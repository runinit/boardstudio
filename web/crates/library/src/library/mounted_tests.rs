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

async fn clean_saved_projects(runtime: &Runtime, ids: &[&str]) {
    for id in ids {
        let _ = runtime.store.delete_project((*id).into()).await;
    }
}

/// Save every `documents` entry into the runtime store. Multi-card list
/// scenarios share this instead of repeating the save block per document.
async fn save_all(runtime: &Runtime, documents: &[&ProjectDoc]) {
    for document in documents {
        runtime
            .store
            .save_document(document, &Default::default())
            .await
            .unwrap();
    }
}

/// Seed `saved` into storage (clearing `clean` first so cross-test storage
/// state cannot leak in), install `current` as the accepted project, and
/// return the runtime. Delete/list scenarios share this instead of repeating
/// the new/clean/save/install sequence.
async fn seeded_runtime(current: ProjectDoc, clean: &[&str], saved: &[&ProjectDoc]) -> Rc<Runtime> {
    let (session, core) = accepted(current);
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    clean_saved_projects(&runtime, clean).await;
    for document in saved {
        runtime
            .store
            .save_document(document, &Default::default())
            .await
            .unwrap();
    }
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    runtime
}

/// `seeded_runtime` plus mounting the project menu and waiting for
/// `expected_cards` saved cards.
async fn seeded_menu(
    current: ProjectDoc,
    clean: &[&str],
    saved: &[&ProjectDoc],
    root_id: &str,
    expected_cards: u32,
) -> (Rc<Runtime>, web_sys::Element, MountedFixture) {
    let runtime = seeded_runtime(current, clean, saved).await;
    let (root, mounted) = mount_menu(runtime.clone(), root_id);
    wait_for_saved_cards(&root, expected_cards).await;
    (runtime, root, mounted)
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

fn advance(session: &mut Session, initial: Vec<Effect>) {
    let mut pending = initial;
    while let Some(effect) = pending.pop() {
        match effect {
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
    let core = CoreEngine::new();
    let effects = session.submit(boardstudio_application::Event::Open {
        operation_id: OperationId(40),
        document,
    });
    advance(&mut session, effects);
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

mod library_list;
mod project_actions;
