use super::setup_guide::{PendingNewKeyboard, SetupGuideRequest};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Event, SessionEpoch, SnapshotToken};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, PartKind, ProjectDoc};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use js_sys::{Array, JsString, Object};
use std::{cell::Cell, rc::Rc, sync::Arc};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

#[derive(Clone, Copy, PartialEq)]
enum ListStatus {
    Loading,
    Ready,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectNameOwner {
    session_epoch: SessionEpoch,
    document_id: String,
}

impl From<&AcceptedSnapshot> for ProjectNameOwner {
    fn from(snapshot: &AcceptedSnapshot) -> Self {
        Self {
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectNameSubmission {
    owner: ProjectNameOwner,
    token: SnapshotToken,
    revision: u64,
}

impl From<&AcceptedSnapshot> for ProjectNameSubmission {
    fn from(snapshot: &AcceptedSnapshot) -> Self {
        Self {
            owner: ProjectNameOwner::from(snapshot),
            token: snapshot.token,
            revision: snapshot.document.revision,
        }
    }
}

impl ProjectNameSubmission {
    fn matches(&self, snapshot: &AcceptedSnapshot) -> bool {
        self.owner == ProjectNameOwner::from(snapshot)
            && self.token == snapshot.token
            && self.revision == snapshot.document.revision
    }
}

#[derive(Clone)]
struct ProjectNameCommitAction {
    runtime: Rc<Runtime>,
    owner: ProjectNameOwner,
    mounted: Rc<Cell<bool>>,
}

impl ProjectNameCommitAction {
    fn commit(&self, value: &str) {
        commit_project_name(&self.runtime, &self.owner, &self.mounted, value);
    }
}

#[derive(Clone)]
struct PreviewKey {
    id: String,
    x: f64,
    y: f64,
    angle: f64,
    width: f64,
    height: f64,
}

struct Preview {
    keys: Vec<PreviewKey>,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

fn project_name(document: &ProjectDoc) -> String {
    if document.name.trim().is_empty() {
        "Untitled keyboard".into()
    } else {
        document.name.clone()
    }
}

fn preview(document: &ProjectDoc) -> Result<Preview, ()> {
    let mut keys = Vec::new();
    for part in &document.parts {
        let definition = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id);
        if !definition.is_some_and(|definition| definition.kind == PartKind::Switch) {
            continue;
        }

        let (width, height) = part
            .keycap
            .or_else(|| definition.and_then(|definition| definition.keycap))
            .map(|size| (size.x, size.y))
            .unwrap_or((18.0, 18.0));
        let key = PreviewKey {
            id: part.id.clone(),
            x: part.pose.at.x,
            y: -part.pose.at.y,
            angle: -part.pose.rotation,
            width,
            height,
        };
        if ![key.x, key.y, key.angle, key.width, key.height]
            .into_iter()
            .all(f64::is_finite)
            || key.width <= 0.0
            || key.height <= 0.0
        {
            return Err(());
        }
        keys.push(key);
    }

    if keys.is_empty() {
        return Ok(Preview {
            keys,
            left: 0.0,
            top: 0.0,
            width: 0.0,
            height: 0.0,
        });
    }

    let mut left = f64::INFINITY;
    let mut top = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut bottom = f64::NEG_INFINITY;
    for key in &keys {
        let angle = key.angle.to_radians();
        let dx = (angle.cos().abs() * key.width + angle.sin().abs() * key.height) / 2.0;
        let dy = (angle.sin().abs() * key.width + angle.cos().abs() * key.height) / 2.0;
        left = left.min(key.x - dx);
        top = top.min(key.y - dy);
        right = right.max(key.x + dx);
        bottom = bottom.max(key.y + dy);
    }
    left -= 8.0;
    top -= 8.0;
    let width = right - left + 8.0;
    let height = bottom - top + 8.0;
    if ![left, top, width, height].into_iter().all(f64::is_finite) || width <= 0.0 || height <= 0.0
    {
        return Err(());
    }
    Ok(Preview {
        keys,
        left,
        top,
        width,
        height,
    })
}

fn sort_saved(documents: &mut [ProjectDoc]) {
    documents.sort_by(|left, right| {
        JsString::from(project_name(left))
            .locale_compare(&project_name(right), &Array::new(), &Object::new())
            .cmp(&0)
    });
}

#[component]
fn KeyboardCard(document: Arc<ProjectDoc>, current: bool, recovery_required: bool) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let name = project_name(&document);
    let preview = preview(&document);
    let action_name = if recovery_required {
        format!("Recover from {name} (discard pending changes)")
    } else {
        format!("Open {name}")
    };
    let document_id = document.id.clone();
    let detail = match &preview {
        Err(()) => "Open to check this keyboard".into(),
        Ok(preview) => format!(
            "{} keys · {}",
            preview.keys.len(),
            if document.boards.len() > 1 {
                format!("{} boards", document.boards.len())
            } else {
                "Single board".into()
            }
        ),
    };
    rsx! {
        article { class: if current { "m1-keyboard-card is-current" } else { "m1-keyboard-card" },
            button {
                class: "m1-keyboard-tile",
                r#type: "button",
                aria_label: "{action_name}",
                aria_current: current.then_some("true"),
                onclick: move |_| {
                    super::close_project_menu();
                    runtime.open_saved(document_id.clone());
                },
                match &preview {
                    Err(()) => rsx! {
                            div { class: "m1-keyboard-preview is-empty",
                                svg { class: "m1-library-keyboard-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                                    path { d: "M2 5h16v11H2ZM5 8h.1M8 8h.1M11 8h.1M14 8h.1M5 11h.1M8 11h.1M11 11h.1M14 11h.1M6 14h8" }
                                }
                                span { "Preview unavailable" }
                            }
                    },
                    Ok(preview) if preview.keys.is_empty() => rsx! {
                            div { class: "m1-keyboard-preview is-empty",
                                svg { class: "m1-library-keyboard-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                                    path { d: "M2 5h16v11H2ZM5 8h.1M8 8h.1M11 8h.1M14 8h.1M5 11h.1M8 11h.1M11 11h.1M14 11h.1M6 14h8" }
                                }
                                span { "No keys placed" }
                            }
                    },
                    Ok(preview) => rsx! {
                        div { class: "m1-keyboard-preview",
                            svg { view_box: "{preview.left} {preview.top} {preview.width} {preview.height}", "aria-hidden": "true",
                                for key in &preview.keys {
                                    rect {
                                        key: "{key.id}",
                                        transform: "translate({key.x} {key.y}) rotate({key.angle})",
                                        x: "{-key.width / 2.0}",
                                        y: "{-key.height / 2.0}",
                                        width: "{key.width}",
                                        height: "{key.height}",
                                        rx: "2",
                                    }
                                }
                            }
                        },
                    }
                }
                span { class: "m1-keyboard-title", "{name}" }
                span { class: "m1-keyboard-detail",
                    "{detail}"
                    if current {
                        span { class: "m1-keyboard-current",
                            svg { class: "m1-library-check-icon", view_box: "0 0 20 20", "aria-hidden": "true", path { d: "m4 10 4 4 8-8" } }
                            "Current"
                            }
                        }
                    },
                }
        }
    }
}

#[component]
pub(super) fn Library() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let project_created = use_context::<Signal<Option<SetupGuideRequest>>>();
    let pending_new = use_context::<Signal<Option<PendingNewKeyboard>>>();
    let new_error = use_context::<Signal<String>>();
    let guide_request_counter = use_signal(|| 0_u64);
    let start_new: Rc<dyn Fn()> = Rc::new({
        let runtime = runtime.clone();
        move || {
            let mut pending_new = pending_new;
            let mut new_error = new_error;
            super::close_project_menu();
            if pending_new().is_some() {
                return;
            }
            new_error.set(String::new());
            match runtime.create_new_keyboard() {
                Ok((project_id, outcome)) => pending_new.set(Some(PendingNewKeyboard {
                    project_id,
                    outcome,
                })),
                Err(error) => new_error.set(error),
            }
        }
    });
    let start_new_card = start_new.clone();
    let start_new_menu = start_new.clone();
    let recovery_required =
        runtime.model().lifecycle == boardstudio_application::Lifecycle::RecoveryRequired;
    let current_snapshot = runtime.model().accepted;
    let current = current_snapshot
        .as_ref()
        .map(|snapshot| snapshot.document.clone());
    let has_current = current.is_some();
    let name_owner = current_snapshot.as_ref().map(ProjectNameOwner::from);
    let current_name = current
        .as_ref()
        .map(|document| document.name.clone())
        .unwrap_or_default();
    let mut project_name = use_signal(|| current_name.clone());
    use_effect(use_reactive!(|name_owner, current_name| {
        let _ = name_owner;
        project_name.set(current_name.clone());
    }));
    let current_project_id = current.as_ref().map(|document| document.id.clone());
    let accepted_identity = current.as_ref().map(|document| document.id.clone());
    let mut saved = use_signal(Vec::<Arc<ProjectDoc>>::new);
    let mut status = use_signal(|| ListStatus::Loading);
    let mut retry = use_signal(|| 0_u64);
    let retry_value = retry();
    let request_generation = use_hook(|| Rc::new(Cell::new(0_u64)));
    let mounted = use_hook(|| Rc::new(Cell::new(true)));
    let name_action = name_owner.clone().map(|owner| ProjectNameCommitAction {
        runtime: runtime.clone(),
        owner,
        mounted: mounted.clone(),
    });
    #[cfg(all(test, target_arch = "wasm32"))]
    PROJECT_NAME_ACTION_PROBE.with(|probe| *probe.borrow_mut() = name_action.clone());
    use_drop({
        let mounted = mounted.clone();
        move || mounted.set(false)
    });

    let list_runtime = runtime.clone();
    let generations = request_generation.clone();
    let mounted_requests = mounted.clone();
    use_effect(use_reactive!(|accepted_identity, retry_value| {
        let _ = (&accepted_identity, retry_value);
        let Some(generation) = generations.get().checked_add(1) else {
            status.set(ListStatus::Failed);
            return;
        };
        generations.set(generation);
        status.set(ListStatus::Loading);
        let runtime = list_runtime.clone();
        let mounted = mounted_requests.clone();
        let generations = generations.clone();
        let request_identity = accepted_identity.clone();
        spawn_local(async move {
            let result = runtime.store.list_documents().await;
            let latest_identity = runtime
                .model()
                .accepted
                .as_ref()
                .map(|snapshot| snapshot.document.id.clone());
            if !mounted.get()
                || generations.get() != generation
                || latest_identity != request_identity
            {
                return;
            }
            match result {
                Ok(mut documents) => {
                    sort_saved(&mut documents);
                    saved.set(documents.into_iter().map(Arc::new).collect());
                    status.set(ListStatus::Ready);
                }
                Err(_) => status.set(ListStatus::Failed),
            }
        });
    }));

    let mut cards = Vec::new();
    if let Some(current) = current {
        cards.push((Arc::clone(&current), true));
        let saved_documents = saved.read();
        cards.extend(
            saved_documents
                .iter()
                .filter(|document| document.id != current.id)
                .map(|document| (Arc::clone(document), false)),
        );
    } else {
        let saved_documents = saved.read();
        cards.extend(
            saved_documents
                .iter()
                .map(|document| (Arc::clone(document), false)),
        );
    }
    let reviung = runtime.clone();
    let sofle = runtime.clone();
    let import = runtime.clone();
    let current_name_for_blur = current_name.clone();
    let rename_action_for_blur = name_action.clone();
    let mut guide_request = project_created;
    let mut guide_request_counter = guide_request_counter;
    let retry_generations = request_generation.clone();
    rsx! {
        section { class: "m1-library", "aria-label": "Your keyboards",
            if has_current {
                section { class: "m1-project-current", "aria-label": "Current project",
                    label { class: "m1-project-title",
                        span { "Current project" }
                        input {
                            "aria-label": "Project name",
                            title: "Rename project",
                            value: "{project_name}",
                            oninput: move |event: FormEvent| project_name.set(event.value()),
                            onkeydown: {
                                let accepted_name = current_name.clone();
                                move |event: KeyboardEvent| {
                                    match event.data().key().to_string().as_str() {
                                        "Enter" => {
                                            event.prevent_default();
                                            if let Some(input) = event
                                                .data()
                                                .try_as_web_event()
                                                .and_then(|event| event.target())
                                                .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                                            {
                                                let _ = input.blur();
                                            }
                                        }
                                        "Escape" => project_name.set(accepted_name.clone()),
                                        _ => {}
                                    }
                                }
                            },
                            onblur: move |_| {
                                let draft = project_name();
                                if draft.trim().is_empty() || draft.trim() == current_name_for_blur {
                                    project_name.set(current_name_for_blur.clone());
                                } else if let Some(action) = rename_action_for_blur.clone() {
                                    action.commit(&draft);
                                }
                            },
                        }
                    }
                }
            }
            header { class: "m1-library-heading",
                h2 { "Your keyboards" if status() == ListStatus::Ready { span { "{cards.len()}" } } }
                span { "Saved in this browser" }
            }
            if status() == ListStatus::Loading {
                p { role: "status", "Loading saved keyboards…" }
            }
            if status() == ListStatus::Failed {
                p { role: "alert", "Saved keyboards could not be loaded. "
                    button { class: "m1-library-text-action", r#type: "button", onclick: move |_| {
                        if let Some(generation) = retry_generations.get().checked_add(1) {
                            retry_generations.set(generation);
                            retry += 1;
                        }
                    }, "Try again" }
                }
            }
            div { class: "m1-keyboard-grid",
                button {
                    class: "m1-keyboard-tile m1-keyboard-new",
                    r#type: "button",
                    aria_label: "Create new keyboard",
                    disabled: pending_new().is_some(),
                    onclick: move |_| start_new_card(),
                    div { class: "m1-keyboard-preview", "aria-hidden": "true", "＋" }
                    span { class: "m1-keyboard-title", "New keyboard" }
                    span { class: "m1-keyboard-detail", "Start with guided setup" }
                }
                for (document, is_current) in cards {
                    KeyboardCard { key: "{document.id}", document, current: is_current, recovery_required }
                }
            }
            div { class: "m1-library-actions",
                button { r#type: "button", disabled: pending_new().is_some(), onclick: move |_| start_new_menu(), "New project" }
                if let Some(project_id) = current_project_id {
                    button { r#type: "button", onclick: move |_| {
                        guide_request_counter += 1;
                        guide_request.set(Some(SetupGuideRequest {
                            project_id: project_id.clone(),
                            request_id: format!("{}-guide-{}", project_id, guide_request_counter()),
                            start_at_project: false,
                        }));
                        super::close_project_menu();
                    }, "Setup guide" }
                }
                button { r#type: "button", onclick: move |_| { super::close_project_menu(); reviung.open_fixture("reviung41"); }, "REVIUNG41 copy" }
                button { r#type: "button", onclick: move |_| { super::close_project_menu(); sofle.open_fixture("sofle"); }, "Sofle v2 copy" }
                label { "Import .boardstudio"
                    input { r#type: "file", accept: ".boardstudio", onchange: move |event: FormEvent| {
                        let Some(input) = event.data().try_as_web_event().and_then(|e| e.target()).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) else { return; };
                        let Some(file) = input.files().and_then(|files| files.get(0)) else { return; };
                        super::close_project_menu();
                        import.import_file(file);
                        input.set_value("");
                    }}
                }
            }
            if pending_new().is_some() { p { role: "status", "Creating keyboard…" } }
            if !new_error().is_empty() { p { role: "alert", "{new_error()}" } }
        }
    }
}

fn renamed_document(document: &ProjectDoc, value: &str) -> Option<ProjectDoc> {
    let name = value.trim();
    if name.is_empty() || name == document.name {
        return None;
    }
    let mut renamed = document.clone();
    renamed.name = name.to_owned();
    Some(renamed)
}

fn commit_project_name(
    runtime: &Rc<Runtime>,
    owner: &ProjectNameOwner,
    mounted: &Rc<Cell<bool>>,
    value: &str,
) {
    if !mounted.get() {
        return;
    }
    let Some(snapshot) = runtime.model().accepted else {
        return;
    };
    if *owner != ProjectNameOwner::from(&snapshot) {
        runtime.report("Project changed while renaming; the project name was not changed.");
        return;
    }
    let Some(document) = renamed_document(&snapshot.document, value) else {
        return;
    };
    let submission = ProjectNameSubmission::from(&snapshot);
    let Some(current) = runtime.model().accepted else {
        return;
    };
    if !mounted.get() || !submission.matches(&current) || *owner != ProjectNameOwner::from(&current)
    {
        runtime.report("Project changed while renaming; the project name was not changed.");
        return;
    }
    let operation_id = runtime.operation();
    runtime.submit(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: submission.revision,
            transaction_id: format!("m1-project-name-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids: vec![document.id.clone()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        },
    });
}

#[cfg(all(test, target_arch = "wasm32"))]
thread_local! {
    static PROJECT_NAME_ACTION_PROBE: std::cell::RefCell<Option<ProjectNameCommitAction>> = const { std::cell::RefCell::new(None) };
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    use super::*;
    use boardstudio_application::{Completion, Effect, OperationId, SaveResult, Session};
    use boardstudio_core::CoreEngine;
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

    fn host() -> Element {
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
            rsx! { Library {} }
        } else {
            rsx! { div { "Library unmounted" } }
        }
    }

    async fn settle() {
        gloo_timers::future::TimeoutFuture::new(80).await;
    }

    async fn wait_outcome(
        runtime: &Rc<Runtime>,
        slot: &crate::operation_outcomes::OutcomeSlot,
    ) -> boardstudio_application::TerminalOutcome {
        for _ in 0..100 {
            crate::runtime::project_name_test_support::run_pending(runtime).await;
            if let Some(outcome) = slot.borrow().clone() {
                return outcome;
            }
            gloo_timers::future::TimeoutFuture::new(20).await;
        }
        panic!(
            "operation did not settle; outcome={:?}, accepted_revision={:?}, status={}",
            *slot.borrow(),
            runtime
                .model()
                .accepted
                .as_ref()
                .map(|snapshot| snapshot.document.revision),
            runtime.status()
        );
    }

    fn submit(
        runtime: &Rc<Runtime>,
        event: impl FnOnce(OperationId) -> Event,
    ) -> crate::operation_outcomes::OutcomeSlot {
        let operation = runtime.operation();
        let slot = crate::runtime::project_name_test_support::observe(runtime, operation);
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
        let mut session = Session::new();
        let mut core = CoreEngine::new();
        let effects = session.submit(boardstudio_application::Event::Open {
            operation_id: OperationId(40),
            document,
        });
        advance(&mut session, &mut core, effects);
        assert!(session.read_model().accepted.is_some());
        (session, core)
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
        let fields = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector_all("input[aria-label='Project name']")
            .unwrap();
        fields
            .item(fields.length().saturating_sub(1))
            .expect("a mounted current project has a name field")
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
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::install(&runtime, session, core);
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
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;

        assert_eq!(field().value(), "Sofle v2");
        let label = field().parent_element().unwrap();
        assert_eq!(
            label.get_attribute("class").as_deref(),
            Some("m1-project-title")
        );
        assert_eq!(label.text_content().unwrap().trim(), "Current project");
        type_value(&field(), "  My custom keyboard  ");
        settle().await;

        let mut unrelated = snapshot.document.as_ref().clone();
        unrelated
            .parameters
            .insert("independent".into(), serde_json::json!(42));
        let unrelated_slot = submit(&runtime, |operation_id| Event::Edit {
            operation_id,
            command: EditCommand {
                base_revision: snapshot.document.revision,
                transaction_id: "project-name-unrelated-edit".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["independent".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(unrelated),
                },
            },
        });
        assert_eq!(
            wait_outcome(&runtime, &unrelated_slot).await,
            boardstudio_application::TerminalOutcome::Completed
        );
        settle().await;

        assert_eq!(
            field().value(),
            "  My custom keyboard  ",
            "an unrelated accepted revision must not discard the menu's active name draft"
        );
        let latest = runtime.model().accepted.unwrap();
        let rename_slot = crate::runtime::project_name_test_support::observe_next(&runtime);
        let _ = field().blur();
        assert_eq!(
            wait_outcome(&runtime, &rename_slot).await,
            boardstudio_application::TerminalOutcome::Completed
        );
        settle().await;
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
        settle().await;
        assert_eq!(field().value(), "Sofle v2");

        let redo_slot = submit(&runtime, |operation_id| Event::Redo { operation_id });
        assert_eq!(
            wait_outcome(&runtime, &redo_slot).await,
            boardstudio_application::TerminalOutcome::Completed
        );
        let redone = runtime.model().accepted.unwrap();
        assert_eq!(redone.document.name, "My custom keyboard");
        settle().await;
        assert_eq!(field().value(), "My custom keyboard");

        type_value(&field(), "Keyboard entered");
        let enter_slot = crate::runtime::project_name_test_support::observe_next(&runtime);
        press(&field(), "Enter");
        assert_eq!(
            wait_outcome(&runtime, &enter_slot).await,
            boardstudio_application::TerminalOutcome::Completed
        );
        let entered = runtime.model().accepted.unwrap();
        assert_eq!(entered.document.name, "Keyboard entered");
        settle().await;

        type_value(&field(), "Canceled draft");
        press(&field(), "Escape");
        settle().await;
        assert_eq!(field().value(), "Keyboard entered");
        let _ = field().blur();
        settle().await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.revision,
            entered.document.revision
        );

        type_value(&field(), "   ");
        let _ = field().blur();
        settle().await;
        assert_eq!(field().value(), "Keyboard entered");
        assert_eq!(
            runtime.model().accepted.unwrap().document.revision,
            entered.document.revision
        );

        type_value(&field(), "Keyboard entered");
        let _ = field().blur();
        settle().await;
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
        settle().await;
        assert_eq!(field().value(), "Sofle v2");
        type_value(&field(), "Stale same-name epoch draft");
        let same_name_reopen = submit(&runtime, |operation_id| Event::Open {
            operation_id,
            document: ProjectDoc::empty("menu-name", "Sofle v2"),
        });
        assert_eq!(
            wait_outcome(&runtime, &same_name_reopen).await,
            boardstudio_application::TerminalOutcome::Completed
        );
        settle().await;
        assert_eq!(field().value(), "Sofle v2");
        let _ = field().blur();
        settle().await;
        assert_eq!(runtime.model().accepted.unwrap().document.revision, 0);

        let reopen_saved = submit(&runtime, |operation_id| Event::Open {
            operation_id,
            document: stored.clone(),
        });
        assert_eq!(
            wait_outcome(&runtime, &reopen_saved).await,
            boardstudio_application::TerminalOutcome::Completed
        );
        settle().await;
        let reopened = runtime.model().accepted.unwrap();
        assert_eq!(reopened.document.name, stored.name);
        assert_eq!(
            reopened.document.parameters.get("independent"),
            Some(&serde_json::json!(42)),
            "reopening the saved renamed document restores its unrelated accepted data"
        );
        assert_eq!(field().value(), stored.name);
        let _ = runtime.store.delete_project("menu-name".into()).await;
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
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::install(&runtime, session, core);
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
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;

        let retained_action = project_name_action();
        assert!(retained_action.mounted.get());
        let mut show_library = seed
            .show_library
            .borrow()
            .expect("host exposes its mounted state");
        show_library.set(false);
        settle().await;
        assert!(
            !retained_action.mounted.get(),
            "Library cleanup retires the exact guard captured by its name action"
        );

        let next_operation = crate::runtime::project_name_test_support::observe_next(&runtime);
        retained_action.commit("Stale retained rename");
        crate::runtime::project_name_test_support::run_pending(&runtime).await;
        settle().await;
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
    async fn project_menu_name_reports_real_persist_failure_without_accepting_the_rename() {
        let (session, core) = accepted(ProjectDoc::empty("menu-name-failure", "Original"));
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::install(&runtime, session, core);
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
        root.set_id("project-name-failure-regression");
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
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;

        crate::runtime::project_name_test_support::fail_next_persist(
            &runtime,
            "injected durable write failure",
        );
        type_value(&field(), "Should not commit");
        let slot = crate::runtime::project_name_test_support::observe_next(&runtime);
        let _ = field().blur();
        assert_eq!(
            wait_outcome(&runtime, &slot).await,
            boardstudio_application::TerminalOutcome::PersistenceFailed(
                "injected durable write failure".into()
            )
        );
        assert_eq!(runtime.model().accepted.unwrap().document.name, "Original");
        assert!(runtime.status().contains("injected durable write failure"));
        let _ = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .remove_child(&root);
    }

    #[wasm_bindgen_test]
    async fn delayed_rename_persist_after_owner_replacement_cannot_update_new_project() {
        let (session, core) = accepted(ProjectDoc::empty("menu-name-delayed", "Original"));
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::install(&runtime, session, core);
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
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;

        let (entered, release) =
            crate::runtime::project_name_test_support::gate_next_persist(&runtime);
        type_value(&field(), "Delayed old rename");
        let slot = crate::runtime::project_name_test_support::observe_next(&runtime);
        let _ = field().blur();
        crate::runtime::project_name_test_support::drive_pending(&runtime);
        entered
            .await
            .expect("the actual Persist effect reaches the gate");
        let (replacement, _replacement_core) =
            accepted(ProjectDoc::empty("menu-name-replacement", "Replacement"));
        crate::runtime::project_name_test_support::replace_session(&runtime, replacement);
        settle().await;
        assert_eq!(field().value(), "Replacement");
        release.send(()).expect("release the captured old save");
        settle().await;
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
}
