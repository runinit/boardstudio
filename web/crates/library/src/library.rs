use super::demo_projects::DEMO_PROJECTS;
use super::setup_guide::{PendingNewKeyboard, SetupGuideRequest};
use crate::project_name::{
    ProjectNameKey, ProjectNameOwner, ProjectNameRejection, project_name_resolver,
};
use crate::runtime::Runtime;
#[cfg(test)]
use boardstudio_application::Event;
#[cfg(test)]
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase};
use boardstudio_core::model::{PartKind, ProjectDoc};
#[cfg(test)]
use boardstudio_web_host::host::ProjectListing;
use boardstudio_web_host::host::UnreadableProject;
use boardstudio_web_ui_shared::controls::{
    BoardButton, BoardButtonDensity, BoardButtonVariant, BoardDialog,
};
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use js_sys::{Array, JsString, Object};
use std::{cell::Cell, rc::Rc, sync::Arc};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

/// Presentation-only browser visibility. The editor remains mounted while browsing.
#[derive(Clone, Copy)]
pub struct ProjectBrowserState(pub Signal<bool>);

#[derive(Clone, Copy, PartialEq)]
enum ListStatus {
    Loading,
    Ready,
    Failed,
}

#[derive(Clone)]
struct ProjectNameCommitAction {
    runtime: Rc<Runtime>,
    owner: ProjectNameOwner,
    mounted: Rc<Cell<bool>>,
    edits: PendingEditSignals<ProjectNameKey>,
    /// The project the latest submitted rename belongs to; its observation is live only
    /// while that project stays accepted.
    submitted_owner: Rc<std::cell::RefCell<Option<ProjectNameOwner>>>,
}

impl ProjectNameCommitAction {
    fn commit(&self, value: &str) {
        match project_name_resolver(&self.runtime, &self.owner, &self.mounted, value) {
            Ok(resolver) => {
                self.edits.begin_field(
                    &self.runtime,
                    ProjectNameKey::Name,
                    "project-name",
                    Some("project name".into()),
                    resolver,
                    value,
                );
                *self.submitted_owner.borrow_mut() = Some(self.owner.clone());
            }
            Err(ProjectNameRejection::OwnerChanged) => self
                .runtime
                .report("Project changed while renaming; the project name was not changed."),
            Err(_) => {}
        }
    }
}

#[derive(Clone, PartialEq)]
struct PreviewKey {
    id: String,
    x: f64,
    y: f64,
    angle: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, PartialEq)]
struct Preview {
    keys: Vec<PreviewKey>,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

fn project_display_name(document: &ProjectDoc) -> String {
    if document.name.trim().is_empty() {
        "Untitled keyboard".into()
    } else {
        document.name.clone()
    }
}

fn matches_project_search(document: &ProjectDoc, query: &str) -> bool {
    let query = query.trim();
    query.is_empty()
        || project_display_name(document)
            .to_lowercase()
            .contains(&query.to_lowercase())
}

fn focus_library_search() {
    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(0).await;
        if let Some(input) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| {
                document
                    .query_selector(".m1-keyboard-search input")
                    .ok()
                    .flatten()
            })
            .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        {
            let _ = input.focus();
        }
    });
}

fn focus_preferences_entry() {
    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(0).await;
        if let Some(button) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id("m1-preferences-entry"))
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = button.focus();
        }
    });
}

fn focus_delete_trigger(project_id: String) {
    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(0).await;
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let Ok(buttons) = document.query_selector_all(".m1-keyboard-delete") else {
            return;
        };
        for index in 0..buttons.length() {
            let Some(button) = buttons
                .item(index)
                .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
            else {
                continue;
            };
            if button.get_attribute("data-project-id").as_deref() == Some(project_id.as_str()) {
                let _ = button.focus();
                return;
            }
        }
    });
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
        JsString::from(project_display_name(left))
            .locale_compare(&project_display_name(right), &Array::new(), &Object::new())
            .cmp(&0)
    });
}

#[derive(Clone)]
struct KeyboardCardSource(Arc<ProjectDoc>);

impl PartialEq for KeyboardCardSource {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[component]
fn KeyboardCard(
    document: KeyboardCardSource,
    current: bool,
    recovery_required: bool,
    deletable: bool,
    #[props(default)] on_select: Option<EventHandler<Arc<ProjectDoc>>>,
    #[props(default)] selected: bool,
) -> Element {
    let document = document.0;
    let runtime = use_context::<Rc<Runtime>>();
    let mut pending_delete = use_context::<Signal<Option<Arc<ProjectDoc>>>>();
    let name = project_display_name(&document);
    let preview = preview(&document);
    let action_name = if recovery_required {
        format!("Recover from {name} (discard pending changes)")
    } else if on_select.is_some() {
        format!("Preview {name}")
    } else {
        format!("Open {name}")
    };
    let document_id = document.id.clone();
    let project_id = document.id.clone();
    let selected_document = document.clone();
    let delete_document = document.clone();
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
        article { class: if selected { "m1-keyboard-card is-selected" } else if current { "m1-keyboard-card is-current" } else { "m1-keyboard-card" },
            "data-project-id": "{project_id}",
            button {
                class: "m1-keyboard-tile",
                r#type: "button",
                aria_label: "{action_name}",
                aria_current: current.then_some("true"),
                aria_pressed: on_select.map(|_| selected),
                onclick: move |_| {
                    if let Some(on_select) = on_select { on_select.call(selected_document.clone()); return; }
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
                    span { class: "m1-keyboard-facts", "{detail}" }
                    if current {
                        span { class: "m1-keyboard-current",
                            svg { class: "m1-library-check-icon", view_box: "0 0 20 20", "aria-hidden": "true", path { d: "m4 10 4 4 8-8" } }
                            "Current"
                            }
                        }
                    },
                }
            }
            if deletable {
                button {
                    class: "m1-keyboard-delete",
                    r#type: "button",
                    aria_label: "Delete {name}",
                    title: "Delete {name}",
                    "data-project-id": "{document.id}",
                    onclick: move |_| pending_delete.set(Some(delete_document.clone())),
                    svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M3 5h14M7 5V3h6v2M5 5l1 12h8l1-12M8 8v6M12 8v6" } }
                }
            }
    }
}

#[component]
fn LibraryPreview(preview: Option<Preview>, fallback: String) -> Element {
    match preview {
        Some(preview) if preview.keys.is_empty() => rsx! {
            div { class: "m1-keyboard-preview is-empty",
                svg { class: "m1-library-keyboard-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                    path { d: "M2 5h16v11H2ZM5 8h.1M8 8h.1M11 8h.1M14 8h.1M5 11h.1M8 11h.1M11 11h.1M14 11h.1M6 14h8" }
                }
                span { "No keys placed" }
            }
        },
        Some(preview) => rsx! {
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
            }
        },
        None => rsx! {
            div { class: "m1-keyboard-preview is-empty",
                svg { class: "m1-library-keyboard-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                    path { d: "M2 5h16v11H2ZM5 8h.1M8 8h.1M11 8h.1M14 8h.1M5 11h.1M8 11h.1M11 11h.1M14 11h.1M6 14h8" }
                }
                span { "{fallback}" }
            }
        },
    }
}

#[component]
fn DemoActionCard() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let browser = try_use_context::<ProjectBrowserState>();
    rsx! {
        article { class: "m1-library-start-card",
            div { class: "m1-library-start-icon m1-library-start-icon--demo",
                svg { view_box: "0 0 96 56", "aria-hidden": "true",
                    rect { x: "4", y: "7", width: "88", height: "42", rx: "4" }
                    for (x, y) in [(10, 13), (20, 13), (30, 13), (40, 13), (50, 13), (60, 13), (70, 13), (80, 13), (14, 24), (24, 24), (34, 24), (44, 24), (54, 24), (64, 24), (74, 24), (20, 35), (30, 35), (40, 35), (50, 35), (60, 35), (70, 35)] {
                        rect { x: "{x}", y: "{y}", width: "6", height: "6", rx: "1" }
                    }
                }
            }
            h2 { "Try a demo" }
            p { "Explore a sample keyboard to see BoardStudio in action." }
            details { class: "m1-demo-picker",
                summary { class: "board-button board-button--primary", "Open demo" }
                div { class: "m1-demo-options", aria_label: "Demo keyboards",
                    for demo in DEMO_PROJECTS {
                        {
                            let document = demo.document();
                            let demo_preview = preview(&document).ok();
                            let runtime = runtime.clone();
                            rsx! {
                                article { class: "m1-demo-option",
                                    LibraryPreview { preview: demo_preview, fallback: demo.name.to_owned() }
                                    div { class: "m1-demo-option-copy",
                                        h3 { "{demo.name}" }
                                        p { "{demo.summary}" }
                                        small { "Editable layout example" }
                                    }
                                    BoardButton { variant: BoardButtonVariant::Primary, onclick: move |_| {
                                        match runtime.open_demo_copy(document.clone()) {
                                            Ok(()) => {
                                                super::close_project_menu();
                                                if let Some(mut browser) = browser { browser.0.set(false); }
                                            }
                                            Err(error) => runtime.report(error),
                                        }
                                    }, "Open demo copy" }
                                }
                            }
                        }
                    }
                    p { class: "m1-demo-note", "These examples demonstrate layout only. Add and verify hardware before fabrication." }
                }
            }
        }
    }
}

#[component]
pub fn Library(
    project_menu: bool,
    #[props(default)] menu_page: Option<Signal<super::ProjectMenuPage>>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let browser = try_use_context::<ProjectBrowserState>();
    let mut selected_project = use_signal(|| None::<Arc<ProjectDoc>>);

    let version = use_context::<Signal<u64>>();
    let _ = version();
    let project_created = use_context::<Signal<Option<SetupGuideRequest>>>();
    let pending_new = use_context::<Signal<Option<PendingNewKeyboard>>>();
    let new_error = use_context::<Signal<String>>();
    let guide_request_counter = use_signal(|| 0_u64);
    let fallback_menu_page = use_signal(|| super::ProjectMenuPage::Project);
    let mut menu_page = menu_page.unwrap_or(fallback_menu_page);
    let start_new: Rc<dyn Fn()> = Rc::new({
        let runtime = runtime.clone();
        move || {
            let mut pending_new = pending_new;
            let mut new_error = new_error;
            super::close_project_menu();
            if let Some(mut browser) = browser {
                browser.0.set(false);
            }
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
    let start_new_menu_top = start_new.clone();
    let recovery_required =
        runtime.model().lifecycle == boardstudio_application::Lifecycle::RecoveryRequired;
    let current_snapshot = runtime.model().accepted;
    let current = current_snapshot
        .as_ref()
        .map(|snapshot| snapshot.document.clone());
    let has_current = current.is_some();
    let current_revision = current
        .as_ref()
        .map(|document| document.revision)
        .unwrap_or_default();
    let current_durability = super::durability_label(&runtime.model().durability);
    let name_owner = current_snapshot.as_ref().map(ProjectNameOwner::from);
    let current_name = current
        .as_ref()
        .map(|document| document.name.clone())
        .unwrap_or_default();
    let mut project_name = use_signal(|| current_name.clone());
    let mut name_dirty = use_signal(|| false);
    let mut name_failure = use_signal(|| None::<String>);
    let name_edits = use_hook(PendingEditSignals::<ProjectNameKey>::new);
    name_edits.bind_field(ProjectNameKey::Name, project_name, name_failure);
    let submitted_name_owner =
        use_hook(|| Rc::new(std::cell::RefCell::new(None::<ProjectNameOwner>)));
    let name_version = version();
    let observed_name = use_hook(|| Rc::new(std::cell::RefCell::new(None)));
    use_effect(use_reactive((&name_owner, &current_name, &name_version), {
        let name_edits = name_edits.clone();
        let submitted_name_owner = submitted_name_owner.clone();
        move |(owner, current_name, _)| {
            let observed = (owner.clone(), current_name.clone());
            let changed = observed_name.borrow().as_ref() != Some(&observed);
            let owner_changed = observed_name
                .borrow()
                .as_ref()
                .is_some_and(|(previous, _)| previous != &owner);
            *observed_name.borrow_mut() = Some(observed);
            let owner_is_live = owner.is_some() && *submitted_name_owner.borrow() == owner;
            name_edits.settle(owner_is_live, |_| current_name.clone());
            let pending = name_edits.is_pending(&ProjectNameKey::Name);
            if owner_changed || (!pending && !*name_dirty.peek() && changed) {
                project_name.set(current_name);
                name_dirty.set(false);
            }
            if owner_changed {
                name_failure.set(None);
            }
        }
    }));
    let current_project_id = current.as_ref().map(|document| document.id.clone());
    let accepted_identity = current.as_ref().map(|document| document.id.clone());
    let mut saved = use_signal(Vec::<Arc<ProjectDoc>>::new);
    let mut unreadable = use_signal(Vec::<UnreadableProject>::new);
    let mut search_query = use_signal(String::new);
    let mut pending_delete = use_signal(|| None::<Arc<ProjectDoc>>);
    let deleting_project = use_signal(|| false);
    let mut delete_error = use_signal(String::new);
    use_context_provider(|| pending_delete);
    let pending_delete_open = pending_delete().is_some();
    let mut status = use_signal(|| ListStatus::Loading);
    let mut retry = use_signal(|| 0_u64);
    let retry_value = retry();
    let request_generation = use_hook(|| Rc::new(Cell::new(0_u64)));
    let mounted = use_hook(|| Rc::new(Cell::new(true)));
    let name_action = name_owner.clone().map(|owner| ProjectNameCommitAction {
        runtime: runtime.clone(),
        owner,
        mounted: mounted.clone(),
        edits: name_edits.clone(),
        submitted_owner: submitted_name_owner.clone(),
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
            #[cfg(all(test, target_arch = "wasm32"))]
            let result = match take_project_list_test_result() {
                Some(result) => {
                    // Hold the test response so the mounted DOM can observe Loading.
                    gloo_timers::future::TimeoutFuture::new(120).await;
                    result.map(|documents| ProjectListing {
                        documents,
                        unreadable: Vec::new(),
                    })
                }
                None => runtime.store.list_projects().await,
            };
            #[cfg(not(all(test, target_arch = "wasm32")))]
            let result = runtime.store.list_projects().await;
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
                Ok(listing) => {
                    unreadable.set(listing.unreadable);
                    let mut documents = listing.documents;
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
    // Resolve selection against the current list so accepted renames and revisions
    // update the handoff instead of leaving a captured document snapshot visible.
    let selected_document = selected_project().and_then(|selected| {
        cards
            .iter()
            .find(|(document, _)| document.id == selected.id)
            .map(|(document, _)| Arc::clone(document))
    });
    let selected_preview = selected_document
        .as_ref()
        .and_then(|document| preview(document).ok());
    let selected_summary = selected_document
        .as_ref()
        .map(|document| {
            let key_count = selected_preview
                .as_ref()
                .map(|preview| preview.keys.len())
                .unwrap_or_default();
            let board_word = if document.boards.len() == 1 {
                "board"
            } else {
                "boards"
            };
            format!(
                "{key_count} keys · {} {board_word} · Revision {}",
                document.boards.len(),
                document.revision
            )
        })
        .unwrap_or_default();
    let saved_count = cards.len();
    let query = search_query();
    cards.retain(|(document, _)| matches_project_search(document, &query));
    let no_search_matches = cards.is_empty() && !query.trim().is_empty();
    let storage_readiness = runtime.storage_readiness();
    let import_disabled = !matches!(storage_readiness, Ok(true));
    let import = runtime.clone();
    let menu_import = import.clone();
    let menu_save_copy = runtime.clone();
    let current_name_for_blur = current_name.clone();
    let selection_name_for_blur = current_name_for_blur.clone();
    let rename_action_for_blur = name_action.clone();
    let selection_rename_action_for_blur = rename_action_for_blur.clone();
    let mut guide_request = project_created;
    let mut guide_request_counter = guide_request_counter;
    let retry_generations = request_generation.clone();
    let pending_delete_action = pending_delete;
    let mut deleting_action = deleting_project;
    let mut delete_error_action = delete_error;
    let retry_after_delete = retry;
    let saved_after_delete = saved;
    let delete_generation = request_generation.clone();
    let delete_runtime = runtime.clone();
    let delete_mounted = mounted.clone();
    let is_settings_page = project_menu && menu_page() == super::ProjectMenuPage::Settings;
    let no_saved_projects = cards.is_empty();
    rsx! {
        section { id: if project_menu { "m1-project-menu-dropdown" } else { "m1-library-landing-content" }, class: if project_menu { "m1-library m1-project-menu-library" } else { "m1-library" }, "aria-label": if project_menu { "Project menu" } else { "Your keyboards" }, "data-page": if project_menu && menu_page() == super::ProjectMenuPage::Settings { "settings" } else { "project" },
            if project_menu {
                header { class: "m1-project-menu-heading",
                    h2 { if menu_page() == super::ProjectMenuPage::Settings { "Workspace settings" } else { "Keyboards" } }
                    button { r#type: "button", aria_label: "Close project menu", onclick: |_| super::close_project_menu(),
                        svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "m5 5 10 10M15 5 5 15" } }
                    }
                }
            }
            if project_menu && !is_settings_page && browser.is_some() {
                BoardButton { class: "m1-browse-projects", onclick: move |_| {
                    if let Some(mut browser) = browser { browser.0.set(true); }
                    super::close_project_menu();
                }, "Browse projects" }
            }
            if is_settings_page {
                div { class: "m1-workspace-preferences",
                    BoardButton {
                        variant: BoardButtonVariant::Quiet,
                        density: BoardButtonDensity::Compact,
                        class: "m1-back-link",
                        onclick: move |_| {
                        menu_page.set(super::ProjectMenuPage::Project);
                        focus_preferences_entry();
                    },
                        svg { view_box: "0 0 16 16", "aria-hidden": "true", path { d: "M13 8H4m4 4-4-4 4-4" } }
                        "Back to project menu"
                    }
                    super::ThemePicker {}
                }
            }
            if let Err(error) = &storage_readiness {
                p { role: "alert", class: "m1-status", "{error}" }
            }
            if project_menu {
                    div { class: "m1-project-menu-actions",
                        button { class: "m1-library-new", r#type: "button", disabled: pending_new().is_some(), onclick: move |_| start_new_menu_top(),
                            svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M10 3v14M3 10h14" } }
                            "New project"
                        }
                        if has_current {
                            button { class: "m1-project-copy-action", r#type: "button", title: "Save project copy…", onclick: move |_| {
                                super::close_project_menu();
                                menu_save_copy.export_project_copy();
                            },
                                svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M10 2v11m-4-4 4 4 4-4M3 14v3h14v-3" } }
                                "Save project copy…"
                            }
                        }
                        label { class: "m1-project-menu-open", "aria-disabled": "{import_disabled}",
                            svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M2 6V4h6l2 2h8v3M2 6v11h14l2-8H5l-3 8" } }
                            "Open project…"
                            input { class: "m1-project-file-input", r#type: "file", accept: ".boardstudio", disabled: import_disabled, onchange: move |event: FormEvent| {
                                let Some(input) = event.data().try_as_web_event().and_then(|e| e.target()).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) else { return; };
                                let Some(file) = input.files().and_then(|files| files.get(0)) else { return; };
                                super::close_project_menu();
                                menu_import.import_file(file);
                                if let Some(mut browser) = browser { browser.0.set(false); }
                                input.set_value("");
                            }}
                        }
                    }
            }
            if has_current && project_menu {
                section { class: "m1-project-current", "aria-label": "Current project",
                    label { class: "m1-project-title",
                        span { "Current project" }
                        input {
                            "aria-label": "Project name",
                            title: "Rename project",
                            value: "{project_name}",
                            oninput: move |event: FormEvent| { name_failure.set(None); name_dirty.set(true); project_name.set(event.value()); },
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
                                        "Escape" => { name_dirty.set(false); project_name.set(accepted_name.clone()); },
                                        _ => {}
                                    }
                                }
                            },
                            onblur: move |_| {
                                if !name_dirty() { return; }
                                name_dirty.set(false);
                                let draft = project_name();
                                if draft.trim().is_empty() {
                                    project_name.set(current_name_for_blur.clone());
                                } else if let Some(action) = rename_action_for_blur.clone() {
                                    action.commit(&draft);
                                }
                            },
                            }
                        }
                    if let Some(message) = name_failure() { p { role: "alert", "{message}" } }
                    span { class: "m1-project-current-status", "Revision {current_revision} · {current_durability}" }
                    }
                }
            div { class: if project_menu { "m1-library-content m1-library-scroll" } else { "m1-library-content" },
                if !project_menu {
                    header { class: "m1-library-intro",
                        h1 { "Your keyboards" }
                        p { "Create, customize and manage your keyboard projects." }
                    }
                }
                header { class: "m1-library-heading",
                    if project_menu {
                        h2 { "Your keyboards" if status() == ListStatus::Ready { span { "{saved_count}" } } }
                        span { "Saved in this browser" }
                    } else {
                        span { class: "m1-library-count", "{saved_count} saved keyboards" }
                    }
                }
                if !project_menu {
                    div { class: "m1-library-start-actions", aria_label: "Start a keyboard project",
                        DemoActionCard {}
                        article { class: "m1-library-start-card",
                            div { class: "m1-library-start-icon",
                                svg { view_box: "0 0 96 56", "aria-hidden": "true",
                                    rect { x: "25", y: "5", width: "46", height: "46", rx: "5" }
                                    path { d: "M48 16v24M36 28h24" }
                                }
                            }
                            h2 { "New keyboard" }
                            p { "Start from a blank canvas and build your keyboard." }
                            BoardButton {
                                variant: BoardButtonVariant::Primary,
                                r#type: "button",
                                aria_label: "Create new keyboard",
                                disabled: pending_new().is_some(),
                                onclick: move |_| start_new(),
                                "Create new"
                            }
                        }
                        article { class: "m1-library-start-card",
                            div { class: "m1-library-start-icon",
                                svg { view_box: "0 0 96 56", "aria-hidden": "true",
                                    path { d: "M25 5h33l14 14v32H25z" }
                                    path { d: "M58 5v14h14M48 41V24m-8 8 8-8 8 8" }
                                }
                            }
                            h2 { "Import project" }
                            p { "Bring in a keyboard file from your computer." }
                            label { class: "board-button board-button--primary m1-library-import-action", "aria-disabled": "{import_disabled}",
                                "Import project"
                                input { class: "m1-project-file-input", r#type: "file", accept: ".boardstudio", disabled: import_disabled, onchange: move |event: FormEvent| {
                                    let Some(input) = event.data().try_as_web_event().and_then(|e| e.target()).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) else { return; };
                                    let Some(file) = input.files().and_then(|files| files.get(0)) else { return; };
                                    super::close_project_menu();
                                    import.import_file(file);
                                    if let Some(mut browser) = browser { browser.0.set(false); }
                                    input.set_value("");
                                }}
                            }
                        }
                    }
                }
                div { class: "m1-library-shelf-heading",
                    h2 { if project_menu { "Saved keyboards" } else { "Your saved keyboards" } }
                }
                if project_menu || saved_count > 0 {
                div { class: "m1-keyboard-search",
                    svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M13.5 13.5 18 18M15 8.5a6.5 6.5 0 1 1-13 0 6.5 6.5 0 0 1 13 0Z" } }
                    input {
                        r#type: "search",
                        aria_label: "Search saved keyboards",
                        placeholder: "Search your keyboards",
                        value: "{query}",
                        oninput: move |event: FormEvent| search_query.set(event.value()),
                    }
                    if !query.is_empty() {
                        BoardButton { r#type: "button", onclick: move |_| search_query.set(String::new()), "Clear search" }
                    }
                }
                }
                if status() == ListStatus::Loading {
                    p { role: "status", "Loading saved keyboards…" }
                }
                if recovery_required && runtime.has_pending_edits() {
                    div { role: "alert", class: "m1-save-error",
                        p { "Your pending edits have not been saved. Download them before reloading if you want to keep them." }
                        button { r#type: "button", onclick: {
                            let runtime = runtime.clone();
                            move |_| runtime.download_pending_edits()
                        }, "Download my pending edits" }
                        button { r#type: "button", onclick: {
                            let runtime = runtime.clone();
                            move |_| runtime.recover_saved()
                        }, "Reload saved version" }
                    }
                }
                if status() == ListStatus::Failed {
                    p { role: "alert", "Saved keyboards could not be loaded. "
                        BoardButton {
                            variant: BoardButtonVariant::Quiet,
                            density: BoardButtonDensity::Compact,
                            class: "m1-library-text-action",
                            onclick: move |_| {
                            if let Some(generation) = retry_generations.get().checked_add(1) {
                                retry_generations.set(generation);
                                retry += 1;
                            }
                        },
                            "Try again"
                        }
                    }
                }
                if !no_saved_projects || project_menu {
                div { class: "m1-keyboard-grid",
                    for (document, is_current) in cards {
                        KeyboardCard { key: "{document.id}", selected: selected_project().as_ref().is_some_and(|selected| selected.id == document.id),
                            document: KeyboardCardSource(document), current: is_current, recovery_required, deletable: true,
                            on_select: (!project_menu).then(|| EventHandler::new(move |document: Arc<ProjectDoc>| {
                                let selected_id = document.id.clone();
                                selected_project.set(Some(document));
                                spawn_local(async move {
                                    gloo_timers::future::TimeoutFuture::new(0).await;
                                    if selected_project.peek().as_ref().is_none_or(|document| document.id != selected_id)
                                        || browser.is_some_and(|browser| !(browser.0)()) { return; }
                                    if let Some(button) = web_sys::window().and_then(|window| window.document())
                                        .and_then(|document| document.get_element_by_id("m1-browser-open-keyboard"))
                                        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok()) {
                                        let _ = button.focus();
                                    }
                                });
                            })),
                        }
                    }
                }
                }
                if !project_menu && no_saved_projects && status() == ListStatus::Ready && query.trim().is_empty() {
                    div { class: "m1-library-empty-state",
                        svg { view_box: "0 0 48 48", "aria-hidden": "true",
                            path { d: "M5 19h10l4 7h10l4-7h10M5 19l3 20h32l3-20" }
                        }
                        h2 { "No keyboards yet" }
                        p { "Get started by trying a demo, creating a new keyboard or importing a project." }
                    }
                }
                for record in unreadable() {
                    div { class: "m1-save-error", role: "alert",
                        strong { "Unreadable saved keyboard: {record.label}" }
                        p { "{record.error}" }
                        button { class: "m1-project-raw-download", r#type: "button", onclick: {
                            let runtime = runtime.clone();
                            let key = record.key.clone();
                            move |_| runtime.download_raw_project(key.clone())
                        }, "Download raw saved data (JSON only)" }
                        button { class: "m1-project-unreadable-delete", r#type: "button", onclick: {
                            let runtime = runtime.clone();
                            let record = record.clone();
                            move |_| {
                                // Token-independent: records we cannot read stay
                                // deletable behind this explicit confirm, with no
                                // list/load/replacement or follow-up save.
                                if !web_sys::window().and_then(|window| window.confirm_with_message(
                                    &format!("Permanently delete what we cannot read for saved keyboard \"{}\"? This removes the unreadable record from this browser and cannot be undone. Download its raw saved data first if you want to keep it.", record.label)
                                ).ok()).unwrap_or(false) { return; }
                                let runtime = runtime.clone();
                                let record = record.clone();
                                spawn_local(async move {
                                    match runtime.store.delete_unreadable_by_key(&record).await {
                                        Ok(()) => retry += 1,
                                        Err(error) => runtime.report(format!("The unreadable keyboard was kept: {error}")),
                                    }
                                });
                            }
                        }, "Delete unreadable keyboard…" }
                    }
                }
                if no_search_matches {
                    p { class: "m1-keyboard-empty", "No keyboards match your search." }
                }
                if !project_menu {
                    div { class: "m1-library-actions",
                        if let Some(project_id) = current_project_id.clone() {
                            button { r#type: "button", onclick: move |_| {
                                guide_request_counter += 1;
                                guide_request.set(Some(SetupGuideRequest {
                                    project_id: project_id.clone(),
                                    request_id: format!("{}-guide-{}", project_id, guide_request_counter()),
                                    start_at_project: false,
                                }));
                                if let Some(mut browser) = browser { browser.0.set(false); }
                                super::close_project_menu();
                            }, "Setup guide" }
                        }
                    }
                }
            }
            if !project_menu {
                aside { class: "m1-library-selection", aria_label: "Keyboard details and starting guidance",
                    if let Some(document) = selected_document {
                        h2 { "{project_display_name(&document)}" }
                        if current_project_id.as_deref() == Some(document.id.as_str()) {
                            details { class: "m1-library-selection-rename-disclosure",
                                summary { "Rename project" }
                                label { class: "m1-library-selection-rename",
                                    span { "Project name" }
                                    input {
                                        "aria-label": "Project name",
                                        title: "Rename project",
                                        value: "{project_name}",
                                        oninput: move |event: FormEvent| { name_failure.set(None); name_dirty.set(true); project_name.set(event.value()); },
                                        onkeydown: {
                                            let accepted_name = current_name.clone();
                                            move |event: KeyboardEvent| {
                                                match event.data().key().to_string().as_str() {
                                                    "Enter" => {
                                                        event.prevent_default();
                                                        if let Some(input) = event.data().try_as_web_event().and_then(|event| event.target()).and_then(|target| target.dyn_into::<HtmlInputElement>().ok()) { let _ = input.blur(); }
                                                    }
                                                    "Escape" => { name_dirty.set(false); project_name.set(accepted_name.clone()); },
                                                    _ => {}
                                                }
                                            }
                                        },
                                        onblur: move |_| {
                                            if !name_dirty() { return; }
                                            name_dirty.set(false);
                                            let draft = project_name();
                                            if draft.trim().is_empty() { project_name.set(selection_name_for_blur.clone()); }
                                            else if let Some(action) = selection_rename_action_for_blur.clone() { action.commit(&draft); }
                                        },
                                    }
                                }
                            }
                        }
                        if let Some(message) = name_failure() { p { role: "alert", "{message}" } }
                        p { class: "m1-library-selection-summary",
                            "{selected_summary}"
                        }
                        p { class: "m1-library-selection-saved", "Saved in this browser" }
                        div { class: "m1-library-selection-preview",
                            LibraryPreview { preview: selected_preview, fallback: "Preview unavailable" }
                        }
                        div { class: "m1-library-selection-actions",
                            BoardButton { "id": "m1-browser-open-keyboard", variant: BoardButtonVariant::Primary, onclick: {
                                let runtime = runtime.clone(); let id = document.id.clone();
                                move |_| { runtime.open_saved(id.clone()); if let Some(mut browser) = browser { browser.0.set(false); } }
                            }, if recovery_required { "Recover (discard pending changes)" } else { "Open keyboard" } }
                            if current_project_id.as_deref() == Some(document.id.as_str()) {
                                BoardButton { onclick: { let runtime = runtime.clone(); move |_| runtime.export_project_copy() }, "Save project copy" }
                            }
                            BoardButton { variant: BoardButtonVariant::Danger, onclick: {
                                let document = document.clone(); move |_| pending_delete.set(Some(document.clone()))
                            }, "Delete keyboard" }
                        }
                    } else {
                        h2 { "Start here" }
                        p { "Choose an option to get started with BoardStudio." }
                        ol { class: "m1-library-start-guide",
                            li { strong { "Try a demo" } p { "Open a sample keyboard to explore features and get familiar with the editor." } }
                            li { strong { "Create a new keyboard" } p { "Start from a blank canvas and bring your ideas to life." } }
                            li { strong { "Import a project" } p { "Load a .boardstudio file from your computer to continue working." } }
                        }
                        p { class: "m1-library-guide-note", "Demo layouts are starting points; add and verify hardware before fabrication." }
                    }
                    p { class: "m1-library-storage-note", "Projects are saved in this browser. Download a .boardstudio copy to move or back them up." }
                }
            }
            if project_menu {
                footer { class: "m1-project-menu-footer",
                    if let Some(project_id) = current_project_id {
                        button { r#type: "button", onclick: move |_| {
                            guide_request_counter += 1;
                            guide_request.set(Some(SetupGuideRequest {
                                project_id: project_id.clone(),
                                request_id: format!("{}-guide-{}", project_id, guide_request_counter()),
                                start_at_project: false,
                            }));
                            super::close_project_menu();
                        },
                            svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M3 3h5l2 2 2-2h5v13h-5l-2 2-2-2H3ZM10 5v13" } }
                            "Setup guide"
                        }
                    }
                    button { id: "m1-preferences-entry", r#type: "button", onclick: move |_| menu_page.set(super::ProjectMenuPage::Settings),
                        svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M3 5h14M3 10h14M3 15h14M6 3v4M14 8v4M8 13v4" } }
                        "Workspace settings"
                    }
                }
            }
            BoardDialog {
                    open: pending_delete_open,
                    dismissible: !deleting_project(),
                    class: "m1-project-delete-dialog",
                    aria_labelledby: "m1-project-delete-title",
                    aria_describedby: "m1-project-delete-description",
                    on_close: move |_| {
                        if let Some(project) = pending_delete() {
                            focus_delete_trigger(project.id.clone());
                        }
                        pending_delete.set(None);
                        delete_error.set(String::new());
                    },
                    if let Some(project) = pending_delete() {
                        h2 { id: "m1-project-delete-title", "Delete “{project_display_name(&project)}”?" }
                    } else {
                        h2 { id: "m1-project-delete-title", "Delete keyboard?" }
                    }
                    p { id: "m1-project-delete-description", "This removes the keyboard saved in this browser. This cannot be undone. Any project copies you downloaded will be kept." }
                    if !delete_error().is_empty() {
                        p { role: "alert", "{delete_error()}" }
                    }
                    div { class: "m1-project-delete-actions",
                        BoardButton {
                            r#type: "button",
                            autofocus: true,
                            disabled: deleting_project(),
                            onclick: move |_| {
                                if !deleting_project() {
                                    if let Some(project) = pending_delete() {
                                        focus_delete_trigger(project.id.clone());
                                    }
                                    pending_delete.set(None);
                                    delete_error.set(String::new());
                                }
                            },
                            "Cancel"
                        }
                        BoardButton { variant: BoardButtonVariant::Danger,
                            class: "m1-project-delete-confirm",
                            r#type: "button",
                            disabled: deleting_project(),
                            onclick: move |_| {
                                if deleting_action() {
                                    return;
                                }
                                let Some(project) = pending_delete_action() else {
                                    return;
                                };
                                deleting_action.set(true);
                                delete_error_action.set(String::new());
                                let runtime = delete_runtime.clone();
                                let mut pending_delete = pending_delete_action;
                                let mut deleting = deleting_action;
                                let mut delete_error = delete_error_action;
                                let mut retry = retry_after_delete;
                                let mut saved = saved_after_delete;
                                let generations = delete_generation.clone();
                                let mounted = delete_mounted.clone();
                                spawn_local(async move {
                                    let result = runtime.delete_saved_project(project.id.clone()).await;
                                    if !mounted.get() {
                                        return;
                                    }
                                    match result {
                                        Ok(()) => {
                                            let remaining = saved
                                                .read()
                                                .iter()
                                                .filter(|saved| saved.id != project.id)
                                                .cloned()
                                                .collect();
                                            saved.set(remaining);
                                            if selected_project.peek().as_ref().is_some_and(|selected| selected.id == project.id) {
                                                selected_project.set(None);
                                            }
                                            pending_delete.set(None);
                                            if let Some(generation) = generations.get().checked_add(1) {
                                                generations.set(generation);
                                                retry += 1;
                                            }
                                            focus_library_search();
                                        }
                                        Err(_) => delete_error.set("This keyboard could not be deleted. Try again.".into()),
                                    }
                                    deleting.set(false);
                                });
                            },
                            if deleting_project() { "Deleting…" } else { "Delete keyboard" }
                        }
                    }
            }
            if pending_new().is_some() { p { role: "status", "Creating keyboard…" } }
            if !new_error().is_empty() { p { role: "alert", "{new_error()}" } }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
type ProjectListTestResult = Result<Vec<ProjectDoc>, boardstudio_web_host::host::PersistError>;

#[cfg(all(test, target_arch = "wasm32"))]
thread_local! {
    static PROJECT_NAME_ACTION_PROBE: std::cell::RefCell<Option<ProjectNameCommitAction>> = const { std::cell::RefCell::new(None) };
    static PROJECT_LIST_TEST_RESULTS: std::cell::RefCell<std::collections::VecDeque<ProjectListTestResult>> = const { std::cell::RefCell::new(std::collections::VecDeque::new()) };
}

#[cfg(all(test, target_arch = "wasm32"))]
fn take_project_list_test_result() -> Option<ProjectListTestResult> {
    PROJECT_LIST_TEST_RESULTS.with(|results| results.borrow_mut().pop_front())
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    use super::*;
    use boardstudio_application::{Completion, Effect, OperationId, SaveResult, Session};
    use boardstudio_core::CoreEngine;
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
                details { class: "m1-project-menu", open: true,
                    summary { "Project" }
                    Library { project_menu }
                }
            }
        } else {
            rsx! { div { "Library unmounted" } }
        }
    }

    fn saved_card_count(root: &web_sys::Element) -> u32 {
        root.query_selector_all(".m1-keyboard-card")
            .unwrap()
            .length()
    }

    /// The saved-keyboard listing's alert, when the listing failed.
    fn list_alert(root: &web_sys::Element) -> Option<web_sys::Element> {
        root.query_selector("[role='alert']").unwrap()
    }

    /// The current dialog for deleting a keyboard is open (its DOM `open` attribute).
    fn delete_dialog_open(root: &web_sys::Element) -> bool {
        root.query_selector(".m1-project-delete-dialog")
            .unwrap()
            .is_some_and(|dialog| dialog.has_attribute("open"))
    }

    /// The title of the only saved keyboard card, when the filtered listing shows one.
    fn saved_card_title(root: &web_sys::Element) -> Option<String> {
        root.query_selector(".m1-keyboard-card .m1-keyboard-title")
            .unwrap()
            .and_then(|title| title.text_content())
    }

    /// Drains held work, then takes one event-loop turn so a render that work scheduled can
    /// run. This is not a wait for any condition. A negative assertion after it is only
    /// drain-protected: no render signal proves the menu re-rendered (recorded in the Outcome).
    async fn drain_and_yield(runtime: &Rc<Runtime>) {
        crate::runtime::project_name_test_support::run_pending(runtime).await;
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

    struct ConfirmFixture {
        window: web_sys::Window,
        original: wasm_bindgen::JsValue,
        answer: Rc<Cell<bool>>,
        message: Rc<std::cell::RefCell<String>>,
        _callback: wasm_bindgen::closure::Closure<dyn FnMut(wasm_bindgen::JsValue) -> bool>,
    }

    impl ConfirmFixture {
        fn new() -> Self {
            let window = web_sys::window().unwrap();
            let original = js_sys::Reflect::get(&window, &"confirm".into()).unwrap();
            let answer = Rc::new(Cell::new(false));
            let result = answer.clone();
            let message = Rc::new(std::cell::RefCell::new(String::new()));
            let prompt = message.clone();
            let callback =
                wasm_bindgen::closure::Closure::<dyn FnMut(wasm_bindgen::JsValue) -> bool>::new(
                    move |value: wasm_bindgen::JsValue| {
                        *prompt.borrow_mut() = value.as_string().unwrap();
                        result.get()
                    },
                );
            js_sys::Reflect::set(&window, &"confirm".into(), callback.as_ref()).unwrap();
            Self {
                window,
                original,
                answer,
                message,
                _callback: callback,
            }
        }
    }

    impl Drop for ConfirmFixture {
        fn drop(&mut self) {
            js_sys::Reflect::set(&self.window, &"confirm".into(), &self.original).unwrap();
        }
    }

    #[wasm_bindgen_test]
    async fn unreadable_record_keeps_healthy_card_and_requires_confirmation_to_delete() {
        PROJECT_LIST_TEST_RESULTS.with(|results| results.borrow_mut().clear());
        let runtime = crate::runtime::project_name_test_support::new_runtime();
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
        crate::runtime::project_name_test_support::install(&runtime, session, core);
        let root = mount_menu(runtime.clone(), "partial-list-mounted-regression");
        wait_until(
            "healthy card and raw recovery actions render together",
            || {
                root.text_content()
                    .unwrap_or_default()
                    .contains("Healthy saved keyboard")
                    && root
                        .query_selector(".m1-project-raw-download")
                        .unwrap()
                        .is_some()
                    && root
                        .query_selector(".m1-project-unreadable-delete")
                        .unwrap()
                        .is_some()
            },
        )
        .await;
        assert!(root.text_content().unwrap().contains("partial-list-future"));
        let raw_action = root
            .query_selector(".m1-project-raw-download")
            .unwrap()
            .unwrap();
        assert!(
            raw_action.text_content().unwrap().contains("JSON only"),
            "raw saved data must not promise an asset-complete archive"
        );
        let confirm = ConfirmFixture::new();
        click(&root, ".m1-project-unreadable-delete");
        drain_and_yield(&runtime).await;
        assert!(
            confirm.message.borrow().contains(&future.id),
            "delete confirmation must name its target"
        );
        assert!(
            runtime
                .store
                .raw_project(&wasm_bindgen::JsValue::from_str(&future.id))
                .await
                .unwrap()
                .is_some()
        );
        confirm.answer.set(true);
        click(&root, ".m1-project-unreadable-delete");
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
        drop(confirm);
        clean_saved_projects(&runtime, &[&healthy.id]).await;
        remove_test_root("partial-list-mounted-regression");
    }

    /// Quota-recovery RED: an unreadable record with no comparable token must
    /// stay deletable behind the explicit "permanently delete what we cannot
    /// read" confirm, and the delete must use the token-independent by-key
    /// path instead of the guarded delete that refuses tokenless records.
    #[wasm_bindgen_test]
    async fn tokenless_unreadable_record_stays_deletable_by_explicit_confirm() {
        let runtime = crate::runtime::project_name_test_support::new_runtime();
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
        crate::runtime::project_name_test_support::install(&runtime, session, core);
        let root = mount_menu(runtime.clone(), "tokenless-unreadable-mounted-regression");
        wait_until(
            "tokenless unreadable record renders with a delete action",
            || {
                root.query_selector(".m1-project-unreadable-delete")
                    .unwrap()
                    .is_some()
            },
        )
        .await;
        let button = root
            .query_selector(".m1-project-unreadable-delete")
            .unwrap()
            .unwrap();
        assert!(
            button.get_attribute("disabled").is_none(),
            "records we cannot read stay deletable without a token"
        );
        let confirm = ConfirmFixture::new();
        click(&root, ".m1-project-unreadable-delete");
        drain_and_yield(&runtime).await;
        assert!(
            confirm
                .message
                .borrow()
                .to_lowercase()
                .contains("permanently delete what we cannot read"),
            "the confirm names the unreadable delete explicitly: {}",
            confirm.message.borrow()
        );
        assert!(
            confirm.message.borrow().contains("tokenless-unreadable"),
            "delete confirmation must name its target"
        );
        confirm.answer.set(true);
        click(&root, ".m1-project-unreadable-delete");
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
        drop(confirm);
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

        let runtime = crate::runtime::project_name_test_support::new_runtime();
        let root = mount_menu(runtime, "damaged-preview-fallback-mounted-regression");
        wait_for_project_cards(&root, 2).await;

        let damaged_card = root
            .query_selector(".m1-keyboard-card button[aria-label='Open Damaged preview keyboard']")
            .unwrap()
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
            root.query_selector(".m1-keyboard-card button[aria-label='Open Healthy keyboard']",)
                .unwrap()
                .is_some(),
            "one damaged preview does not hide a healthy saved card"
        );

        PROJECT_LIST_TEST_RESULTS.with(|results| results.borrow_mut().clear());
        remove_test_root("damaged-preview-fallback-mounted-regression");
    }

    #[wasm_bindgen_test]
    async fn saved_project_list_loading_error_and_retry_recover_in_mounted_library() {
        let (session, core) = accepted(ProjectDoc::empty("list-retry-current", "Current keyboard"));
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::install(&runtime, session, core);
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

        let root = mount_menu(runtime, "saved-project-list-retry-mounted-regression");
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
            root.query_selector_all(".m1-keyboard-card")
                .unwrap()
                .length(),
            1,
            "the current project remains available while listing fails"
        );

        click(&root, "[role='alert'] button");
        wait_for_project_cards(&root, 2).await;
        assert!(root.query_selector("[role='alert']").unwrap().is_none());
        assert!(root.text_content().unwrap().contains("Recovered keyboard"));
        assert!(
            root.query_selector(".m1-keyboard-card [aria-current='true']")
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
            crate::runtime::project_name_test_support::run_pending(runtime).await;
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

    fn mount_menu(runtime: Rc<Runtime>, root_id: &str) -> web_sys::Element {
        mount_library(runtime, root_id, true)
    }

    fn mount_library(runtime: Rc<Runtime>, root_id: &str, project_menu: bool) -> web_sys::Element {
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
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        root
    }

    #[wasm_bindgen_test]
    async fn import_waits_for_storage_initialization() {
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::set_storage_initializing(&runtime, true);
        let root = mount_library(runtime.clone(), "import-readiness", false);
        wait_until("import input renders", || {
            root.query_selector("input.m1-project-file-input")
                .unwrap()
                .is_some()
        })
        .await;
        let input = root
            .query_selector("input.m1-project-file-input")
            .unwrap()
            .unwrap();
        assert!(
            input.has_attribute("disabled"),
            "import must be disabled while storage initializes"
        );
        crate::runtime::project_name_test_support::set_storage_initializing(&runtime, false);
        wait_until(
            "import becomes enabled after storage initialization",
            || !input.has_attribute("disabled"),
        )
        .await;
        crate::runtime::project_name_test_support::fail_storage_initialization(
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
        let menu = mount_menu(runtime, "import-readiness-menu");
        wait_until("menu import renders disabled after storage failure", || {
            menu.query_selector("input.m1-project-file-input:disabled")
                .unwrap()
                .is_some()
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
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::install(&runtime, session, core);
        PROJECT_LIST_TEST_RESULTS.with(|results| {
            results.borrow_mut().clear();
            results.borrow_mut().push_back(Ok(vec![ProjectDoc::empty(
                "browser-selection-other",
                "Other keyboard",
            )]));
        });
        let root = mount_library(runtime.clone(), "browser-selection-test-root", false);
        wait_for_saved_cards(&root, 2).await;
        click(&root, "button[aria-label='Preview Other keyboard']");
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
            root.query_selector(".m1-library-selection")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap()
                .contains("Other keyboard")
        );
        click(&root, "button[aria-label='Preview Current keyboard']");
        wait_until("current keyboard preview is selected", || {
            root.query_selector(".m1-library-selection > h2")
                .unwrap()
                .and_then(|heading| heading.text_content())
                .as_deref()
                == Some("Current keyboard")
        })
        .await;
        click(&root, ".m1-library-selection-rename-disclosure > summary");
        type_value(&field(), "Renamed current");
        let rename_slot = crate::runtime::project_name_test_support::observe_next(&runtime);
        let _ = field().blur();
        assert_eq!(
            wait_outcome(&runtime, &rename_slot).await,
            boardstudio_application::TerminalOutcome::Completed
        );
        wait_until("selected preview follows the accepted rename", || {
            root.query_selector(".m1-library-selection > h2")
                .unwrap()
                .and_then(|heading| heading.text_content())
                .as_deref()
                == Some("Renamed current")
        })
        .await;
        PROJECT_LIST_TEST_RESULTS.with(|results| results.borrow_mut().clear());
        root.remove();
    }

    fn click(root: &web_sys::Element, selector: &str) {
        root.query_selector(selector)
            .unwrap()
            .unwrap_or_else(|| panic!("missing {selector}"))
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
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
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::install(&runtime, session, core);
        let root = mount_menu(runtime, "project-copy-mounted-regression");
        wait_until("the Project menu renders its copy action", || {
            root.query_selector(".m1-project-copy-action")
                .unwrap()
                .is_some()
        })
        .await;

        let copy = root
            .query_selector(".m1-project-copy-action")
            .unwrap()
            .expect("the Project menu exposes Save project copy");
        assert_eq!(copy.text_content().unwrap().trim(), "Save project copy…");

        copy.dyn_into::<web_sys::HtmlElement>().unwrap().click();
        // The menu starts open, so "closed" is false before the click and proves its effect.
        wait_until("starting a project copy closes the menu", || {
            root.query_selector("details.m1-project-menu")
                .unwrap()
                .is_some_and(|menu| !menu.has_attribute("open"))
        })
        .await;
        let menu = root
            .query_selector("details.m1-project-menu")
            .unwrap()
            .unwrap();
        assert!(
            !menu.has_attribute("open"),
            "starting a project copy closes the menu"
        );
        remove_test_root("project-copy-mounted-regression");
    }

    #[wasm_bindgen_test]
    async fn current_project_delete_cancel_and_confirm_use_the_observed_replacement_path() {
        remove_test_root("delete-current-mounted-regression");
        remove_test_root("delete-current-failure-regression");
        let document = ProjectDoc::empty("delete-current-ui", "Current board");
        let (session, core) = accepted(document.clone());
        let runtime = crate::runtime::project_name_test_support::new_runtime();
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
        crate::runtime::project_name_test_support::install(&runtime, session, core);
        let root = mount_menu(runtime.clone(), "delete-current-mounted-regression");
        wait_for_saved_cards(&root, 1).await;

        let trigger = root
            .query_selector(".m1-keyboard-delete[data-project-id='delete-current-ui']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        trigger.click();
        wait_until("the delete dialog opens", || delete_dialog_open(&root)).await;
        let dialog = root
            .query_selector(".m1-project-delete-dialog")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlDialogElement>()
            .unwrap();
        assert!(dialog.open());
        assert!(
            dialog
                .text_content()
                .unwrap()
                .contains("Delete “Current board”?")
        );
        dialog
            .dispatch_event(&DomEvent::new("cancel").unwrap())
            .unwrap();
        wait_until("Cancel closes the delete dialog", || {
            !delete_dialog_open(&root)
        })
        .await;
        assert!(!dialog.open());
        // Cancel submits nothing: drain held work, then the store must still hold the record.
        crate::runtime::project_name_test_support::run_pending(&runtime).await;
        assert!(
            runtime
                .store
                .load_document("delete-current-ui".into())
                .await
                .unwrap()
                .is_some()
        );
        wait_until(
            "Escape restores focus to the initiating delete button",
            || {
                web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .active_element()
                    .and_then(|element| element.get_attribute("data-project-id"))
                    .as_deref()
                    == Some("delete-current-ui")
            },
        )
        .await;
        let active = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .active_element()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        assert_eq!(
            active.get_attribute("data-project-id").as_deref(),
            Some("delete-current-ui"),
            "Escape restores focus to the initiating delete button"
        );

        trigger.click();
        wait_until("the delete dialog reopens", || delete_dialog_open(&root)).await;
        click(&root, ".m1-project-delete-confirm");
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
        assert!(!dialog.open());
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
        let runtime = crate::runtime::project_name_test_support::new_runtime();
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
        crate::runtime::project_name_test_support::install(&runtime, session, core);
        let root = mount_menu(runtime.clone(), "delete-current-sort-regression");
        wait_for_saved_cards(&root, 3).await;
        click(
            &root,
            ".m1-keyboard-delete[data-project-id='delete-sort-current']",
        );
        wait_until("the delete dialog opens", || delete_dialog_open(&root)).await;
        click(&root, ".m1-project-delete-confirm");

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
        let runtime = crate::runtime::project_name_test_support::new_runtime();
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
        crate::runtime::project_name_test_support::install(&runtime, session, core);
        crate::runtime::project_name_test_support::fail_next_persist(
            &runtime,
            "replacement storage failed",
        );
        let root = mount_menu(runtime.clone(), "delete-current-failure-regression");
        wait_for_saved_cards(&root, 2).await;
        click(
            &root,
            ".m1-keyboard-delete[data-project-id='delete-failure-current']",
        );
        wait_until("the delete dialog opens", || delete_dialog_open(&root)).await;
        click(&root, ".m1-project-delete-confirm");

        wait_until(
            "a failed replacement stays actionable in the dialog",
            || {
                root.query_selector(".m1-project-delete-dialog [role='alert']")
                    .unwrap()
                    .is_some()
            },
        )
        .await;
        assert!(
            root.query_selector(".m1-project-delete-dialog")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlDialogElement>()
                .unwrap()
                .open()
        );
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
        click(&root, ".m1-project-delete-confirm");
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
        let runtime = crate::runtime::project_name_test_support::new_runtime();
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
        crate::runtime::project_name_test_support::install(&runtime, session, core);

        let initial = runtime.model().accepted.unwrap();
        let mut renamed = (*initial.document).clone();
        renamed.name = "Current board edited".into();
        let edit = submit(&runtime, |operation_id| {
            crate::runtime::project_name_test_support::fixed_command_event(
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

    fn name_field_rendered() -> bool {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("input[aria-label='Project name']")
            .unwrap()
            .is_some()
    }

    fn search_field() -> HtmlInputElement {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(".m1-keyboard-search input[aria-label='Search saved keyboards']")
            .unwrap()
            .expect("the saved keyboard search is mounted")
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
        wait_for_saved_cards(&root, 4).await;

        let project_menu = root
            .query_selector(".m1-project-menu-library")
            .unwrap()
            .expect("menu instance uses the dropdown composition");
        let mut menu_child = project_menu.first_element_child();
        for expected in [
            "m1-project-menu-heading",
            "m1-project-menu-actions",
            "m1-project-current",
            "m1-library-content m1-library-scroll",
            "m1-project-menu-footer",
            "board-dialog m1-project-delete-dialog",
        ] {
            let child = menu_child.expect("expected direct menu child");
            assert_eq!(child.get_attribute("class").as_deref(), Some(expected));
            menu_child = child.next_element_sibling();
        }
        assert!(menu_child.is_none(), "unexpected direct menu child");
        assert!(
            root.query_selector(".m1-project-menu-heading h2")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap()
                .contains("Keyboards")
        );
        assert!(
            root.query_selector(".m1-project-menu-actions .m1-library-new")
                .unwrap()
                .is_some()
        );
        assert!(
            root.query_selector(".m1-project-menu-actions .m1-project-menu-open")
                .unwrap()
                .is_some()
        );
        assert!(
            root.query_selector(".m1-project-menu-footer button")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap()
                .contains("Setup guide")
        );

        let search = search_field();
        assert_eq!(
            search.get_attribute("placeholder").as_deref(),
            Some("Search your keyboards")
        );
        type_value(&search, "  sOfLe  ");
        wait_until("the trimmed search leaves the current keyboard", || {
            saved_card_count(&root) == 1 && saved_card_title(&root).as_deref() == Some("Sofle v2")
        })
        .await;
        assert_eq!(
            root.query_selector_all(".m1-keyboard-card")
                .unwrap()
                .length(),
            1,
            "trimmed case-insensitive search includes the current keyboard"
        );
        assert_eq!(
            root.query_selector(".m1-keyboard-card .m1-keyboard-title")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Sofle v2")
        );
        type_value(&search, "alpha");
        wait_until("the alpha search shows Alpha Board", || {
            saved_card_count(&root) == 1
                && saved_card_title(&root).as_deref() == Some("Alpha Board")
        })
        .await;
        assert_eq!(
            root.query_selector_all(".m1-keyboard-card")
                .unwrap()
                .length(),
            1
        );
        assert_eq!(
            root.query_selector(".m1-keyboard-card .m1-keyboard-title")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Alpha Board")
        );
        type_value(&search, "untitled");
        wait_until("the untitled search shows the Untitled fallback", || {
            saved_card_count(&root) == 1
                && saved_card_title(&root).as_deref() == Some("Untitled keyboard")
        })
        .await;
        assert_eq!(
            root.query_selector_all(".m1-keyboard-card")
                .unwrap()
                .length(),
            1
        );
        assert_eq!(
            root.query_selector(".m1-keyboard-card .m1-keyboard-title")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
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
        let clear_search = root
            .query_selector(".m1-keyboard-search button")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        clear_search.click();
        wait_until("clearing the search restores all four saved cards", || {
            search.value().is_empty() && saved_card_count(&root) == 4
        })
        .await;
        assert_eq!(search.value(), "");
        assert_eq!(
            root.query_selector_all(".m1-keyboard-card")
                .unwrap()
                .length(),
            4
        );

        assert_eq!(field().value(), "Sofle v2");
        let label = field().parent_element().unwrap();
        assert_eq!(
            label.get_attribute("class").as_deref(),
            Some("m1-project-title")
        );
        assert_eq!(label.text_content().unwrap().trim(), "Current project");
        type_value(&field(), "  My custom keyboard  ");

        let mut unrelated = snapshot.document.as_ref().clone();
        unrelated
            .parameters
            .insert("independent".into(), serde_json::json!(42));
        let unrelated_slot = submit(&runtime, |operation_id| {
            crate::runtime::project_name_test_support::fixed_command_event(
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
        let rename_slot = crate::runtime::project_name_test_support::observe_next(&runtime);
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
        let enter_slot = crate::runtime::project_name_test_support::observe_next(&runtime);
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

        let next_operation = crate::runtime::project_name_test_support::observe_next(&runtime);
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
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("the Library mounts the name field", name_field_rendered).await;

        use crate::runtime::project_name_test_support as support;
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
        let landed = wait_for_step(
            "the gated rename lands as Committed",
            WAIT_BOUND_MS,
            async || {
                support::run_pending(&runtime).await;
                runtime
                    .model()
                    .accepted
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.document.name == "Committed")
            },
        )
        .await;
        if let Err(message) = landed {
            let model = runtime.model();
            panic!(
                "{message}; accepted_name={:?}, lifecycle={:?}, durability={:?}, status={}",
                model
                    .accepted
                    .as_ref()
                    .map(|snapshot| &snapshot.document.name),
                model.lifecycle,
                model.durability,
                runtime.status()
            );
        }
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
    ) -> (Rc<crate::runtime::Runtime>, web_sys::Element) {
        let (session, core) = accepted(ProjectDoc::empty(project_id, "Original"));
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::install(&runtime, session, core);
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
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("the Library mounts the name field", name_field_rendered).await;
        (runtime, root)
    }

    fn name_alert(root: &web_sys::Element) -> Option<String> {
        root.query_selector("p[role='alert']")
            .unwrap()
            .and_then(|alert| alert.text_content())
    }

    #[wasm_bindgen_test]
    async fn a_failed_unchanged_project_name_restores_the_accepted_name_with_an_inline_failure() {
        use crate::runtime::project_name_test_support as support;
        let (runtime, root) = mounted_name_field("menu-name-unchanged-failure").await;
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
        use crate::runtime::project_name_test_support as support;
        let (runtime, root) = mounted_name_field("menu-name-newer-failure").await;
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
        use crate::runtime::project_name_test_support as support;
        let (runtime, root) = mounted_name_field("menu-name-latest-observation").await;
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
        wait_until("the Library mounts the name field", name_field_rendered).await;

        let (mut entered, release) =
            crate::runtime::project_name_test_support::gate_next_persist(&runtime);
        type_value(&field(), "Delayed old rename");
        let slot = crate::runtime::project_name_test_support::observe_next(&runtime);
        let _ = field().blur();
        wait_until_step("the actual Persist effect reaches the gate", async || {
            crate::runtime::project_name_test_support::drive_pending(&runtime);
            matches!(entered.try_recv(), Ok(Some(())))
        })
        .await;
        let (replacement, _replacement_core) =
            accepted(ProjectDoc::empty("menu-name-replacement", "Replacement"));
        crate::runtime::project_name_test_support::replace_session(&runtime, replacement);
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
}

#[cfg(all(test, target_arch = "wasm32"))]
mod queued_rename_tests {
    use super::*;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_web_ui_shared::test_wait::wait_until_step;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    async fn library_rename_queued_after_parts_edit_preserves_both_and_undo_order() {
        let runtime = support::new_runtime();
        let mut document = ProjectDoc::empty("queued-rename", "Original");
        document.definitions.push(serde_json::from_value(serde_json::json!({
            "id": "switch", "name": "Original switch", "kind": "switch", "courtyard": [], "pads": []
        })).unwrap());
        support::open_document(&runtime, document).await;
        let accepted = runtime.model().accepted.unwrap();
        let owner = ProjectNameOwner::from(&accepted);
        let mut replacement = accepted.document.as_ref().clone();
        replacement.definitions[0].name = "Updated switch".into();
        let (mut entered, release) = support::gate_next_core_reply(&runtime);
        support::submit_fixed_command(
            &runtime,
            EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: "parts-name".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["switch".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
            },
        );
        wait_until_step("the parts edit reaches its gated Core reply", async || {
            support::drive_pending(&runtime);
            matches!(entered.try_recv(), Ok(Some(())))
        })
        .await;
        ProjectNameCommitAction {
            runtime: runtime.clone(),
            owner,
            mounted: Rc::new(Cell::new(true)),
            edits: PendingEditSignals::new(),
            submitted_owner: Rc::default(),
        }
        .commit("Renamed");
        support::drive_pending(&runtime);
        release.send(()).unwrap();
        wait_until_step(
            "the parts edit and the queued rename both land",
            async || {
                support::run_pending(&runtime).await;
                let result = runtime.model().accepted.unwrap();
                result.document.name == "Renamed"
                    && result.document.definitions[0].name == "Updated switch"
            },
        )
        .await;
        let result = runtime.model().accepted.unwrap();
        assert_eq!(result.document.name, "Renamed");
        assert_eq!(result.document.definitions[0].name, "Updated switch");
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        wait_until_step("the first undo reverts only the rename", async || {
            support::run_pending(&runtime).await;
            let result = runtime.model().accepted.unwrap();
            result.document.name == "Original"
                && result.document.definitions[0].name == "Updated switch"
        })
        .await;
        let result = runtime.model().accepted.unwrap();
        assert_eq!(result.document.name, "Original");
        assert_eq!(result.document.definitions[0].name, "Updated switch");
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        wait_until_step("the second undo reverts the parts edit", async || {
            support::run_pending(&runtime).await;
            runtime.model().accepted.unwrap().document.definitions[0].name == "Original switch"
        })
        .await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.definitions[0].name,
            "Original switch"
        );
    }
}
