use super::demo_projects::DEMO_PROJECTS;
use super::setup_guide::{PendingNewKeyboard, SetupGuideRequest};
use crate::project_name::{ProjectNameOwner, ProjectNameRejection, project_name_resolver};
use crate::runtime::Runtime;
use boardstudio_model::model::{PartKind, ProjectDoc};
use boardstudio_web_host::host::UnreadableProject;
use boardstudio_web_ui_shared::inspector_fields::TextCommitField;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    alert_dialog::{
        AlertDialog, AlertDialogActions, AlertDialogCancel, AlertDialogDescription,
        AlertDialogTitle,
    },
    button::{Button, ButtonSize, ButtonVariant},
    callout::Callout,
    card::Card,
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    empty::{Empty, EmptyDescription, EmptyTitle},
    field::FieldError,
    input::{Input, InputType},
    input_group::{InputGroup, InputGroupAddon, InputGroupInput},
    spinner::Spinner,
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use js_sys::{Array, JsString, Object};
use std::{cell::Cell, rc::Rc, sync::Arc};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

mod header_actions;
pub use header_actions::{ProjectHeaderMenu, use_project_tools};

#[derive(Clone, PartialEq)]
enum ProjectTool {
    Rename(String),
    Demo,
}

#[derive(Clone, Copy)]
struct ProjectToolRequest(Signal<Option<ProjectTool>>);

/// Presentation-only browser visibility. The editor remains mounted while browsing.
#[derive(Clone, Copy)]
pub struct ProjectBrowserState(pub Signal<bool>);

#[derive(Clone, Copy, PartialEq)]
enum ListStatus {
    Loading,
    Ready,
    Failed,
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

fn project_display_name(doc: &ProjectDoc) -> String {
    if doc.name.trim().is_empty() {
        "Untitled keyboard".into()
    } else {
        doc.name.clone()
    }
}

fn matches_project_search(doc: &ProjectDoc, query: &str) -> bool {
    let q = query.trim();
    q.is_empty()
        || project_display_name(doc)
            .to_lowercase()
            .contains(&q.to_lowercase())
}

fn focus_element_by_selector(selector: &'static str) {
    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(0).await;
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.query_selector(selector).ok().flatten())
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = el.focus();
        }
    });
}

fn preview(doc: &ProjectDoc) -> Option<Preview> {
    let mut keys = Vec::new();
    for part in &doc.parts {
        let def = doc.definitions.iter().find(|d| d.id == part.definition_id);
        if !def.is_some_and(|d| d.kind == PartKind::Switch) {
            continue;
        }
        let (width, height) = part
            .keycap
            .or_else(|| def.and_then(|d| d.keycap))
            .map_or((18.0, 18.0), |s| (s.x, s.y));
        let k = PreviewKey {
            id: part.id.clone(),
            x: part.pose.at.x,
            y: -part.pose.at.y,
            angle: -part.pose.rotation,
            width,
            height,
        };
        if ![k.x, k.y, k.angle, k.width, k.height]
            .into_iter()
            .all(f64::is_finite)
            || k.width <= 0.0
            || k.height <= 0.0
        {
            return None;
        }
        keys.push(k);
    }
    if keys.is_empty() {
        return Some(Preview {
            keys,
            left: 0.0,
            top: 0.0,
            width: 0.0,
            height: 0.0,
        });
    }
    let (mut l, mut t, mut r, mut b) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for k in &keys {
        let a = k.angle.to_radians();
        let dx = (a.cos().abs() * k.width + a.sin().abs() * k.height) / 2.0;
        let dy = (a.sin().abs() * k.width + a.cos().abs() * k.height) / 2.0;
        (l, t, r, b) = (
            l.min(k.x - dx),
            t.min(k.y - dy),
            r.max(k.x + dx),
            b.max(k.y + dy),
        );
    }
    let (left, top, width, height) = (l - 8.0, t - 8.0, r - l + 16.0, b - t + 16.0);
    ([left, top, width, height].into_iter().all(f64::is_finite) && width > 0.0 && height > 0.0)
        .then_some(Preview {
            keys,
            left,
            top,
            width,
            height,
        })
}

fn sort_saved(docs: &mut [ProjectDoc]) {
    docs.sort_by(|l, r| {
        JsString::from(project_display_name(l))
            .locale_compare(&project_display_name(r), &Array::new(), &Object::new())
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
fn LibraryPreview(preview: Option<Preview>, fallback: String) -> Element {
    let frame = "grid place-items-center w-full h-[108px] mb-1 p-3.5 rounded bg-canvas text-geometry-key [&_svg]:w-full [&_svg]:h-full [&_svg]:max-w-full [&_rect]:fill-geometry-body [&_rect]:stroke-geometry-key [&_rect]:stroke-[1] max-[760px]:h-[92px] max-[760px]:p-2.5 [#m1-library-landing-content_&]:h-[clamp(120px,12vw,190px)]";
    let empty_cls = format!(
        "{frame} content-center gap-2 text-muted-foreground text-xs [&_svg]:!w-[30px] [&_svg]:!h-[30px] [&_svg]:justify-self-center"
    );
    match preview {
        Some(p) if !p.keys.is_empty() => rsx! {
            div { class: "{frame}",
                svg { view_box: "{p.left} {p.top} {p.width} {p.height}", "aria-hidden": "true",
                    for k in &p.keys {
                        rect { key: "{k.id}", transform: "translate({k.x} {k.y}) rotate({k.angle})",
                            x: "{-k.width / 2.0}", y: "{-k.height / 2.0}", width: "{k.width}", height: "{k.height}", rx: "2" }
                    }
                }
            }
        },
        other => rsx! {
            div { class: "{empty_cls}",
                svg { class: "fill-none stroke-current stroke-[1.5] [stroke-linecap:round] [stroke-linejoin:round]", view_box: "0 0 20 20", "aria-hidden": "true",
                    path { d: "M2 5h16v11H2ZM5 8h.1M8 8h.1M11 8h.1M14 8h.1M5 11h.1M8 11h.1M11 11h.1M14 11h.1M6 14h8" }
                }
                span { if other.is_some() { "No keys placed" } else { "{fallback}" } }
            }
        },
    }
}

#[component]
fn KeyboardCard(
    document: KeyboardCardSource,
    current: bool,
    selected: bool,
    on_select: EventHandler<Arc<ProjectDoc>>,
) -> Element {
    let doc = document.0;
    let mut pending_delete = use_context::<Signal<Option<Arc<ProjectDoc>>>>();
    let name = project_display_name(&doc);
    let p = preview(&doc);
    let action_name = format!("Preview {name}");
    let detail = match &p {
        None => "Open to check this keyboard".into(),
        Some(p) => format!(
            "{} keys · {}",
            p.keys.len(),
            if doc.boards.len() > 1 {
                format!("{} boards", doc.boards.len())
            } else {
                "Single board".into()
            }
        ),
    };
    let (sel_doc, del_doc, del_id, del_name) =
        (doc.clone(), doc.clone(), doc.id.clone(), name.clone());
    rsx! {
        Card { class: if selected { "relative min-w-0 p-0 gap-0 bg-selected text-selected-foreground" } else { "relative min-w-0 p-0 gap-0" },
            div { "data-project-id": "{doc.id}",
                Button { variant: ButtonVariant::Ghost, class: "flex w-full h-auto min-h-11 flex-col items-stretch justify-start gap-[5px] p-3 text-left whitespace-normal",
                    button_type: "button", aria_label: "{action_name}", aria_current: current.then_some("true".to_owned()), aria_pressed: Some(selected),
                    onclick: move |_| on_select.call(sel_doc.clone()),
                    LibraryPreview { preview: p, fallback: "Preview unavailable".to_owned() }
                    span { class: "max-w-full px-1.5 text-foreground text-[13px] font-semibold [overflow-wrap:anywhere] [#m1-library-landing-content_&]:text-lg", "{name}" }
                    span { class: "flex flex-wrap items-center gap-x-2 gap-y-1 px-1.5 text-muted-foreground text-[14px]",
                        span { "{detail}" }
                        if current { span { class: "inline-flex items-center gap-0.5 text-primary", "Current" } }
                    }
                }
            }
            Tooltip { class: "library-card-delete",
                TooltipTrigger { r#as: move |mut attrs: Vec<Attribute>| {
                    let del_doc = del_doc.clone();
                    attrs.push(Attribute::new("data-project-id", del_id.clone(), None, false));
                    rsx! {
                        Button { attributes: attrs, variant: ButtonVariant::Ghost, size: ButtonSize::Icon, button_type: "button",
                            aria_label: format!("Delete {del_name}"), aria_haspopup: "dialog", aria_controls: "library-project-delete",
                            aria_expanded: pending_delete().as_ref().is_some_and(|p| p.id == del_id),
                            onclick: move |_| pending_delete.set(Some(del_doc.clone())),
                            svg { class: "size-4 fill-none stroke-current stroke-[1.5]", view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M3 5h14M7 5V3h6v2M5 5l1 12h8l1-12M8 8v6M12 8v6" } }
                        }
                    }
                } }
                TooltipContent { "Delete {name}" }
            }
        }
    }
}

#[component]
fn DemoActionCard(mut open: Signal<bool>) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let browser = try_use_context::<ProjectBrowserState>();
    let content_id = format!(
        "library-demo-options-{}",
        dioxus::core::current_scope_id().0
    );
    let trigger_id = content_id.clone();
    rsx! {
        Card { class: "min-w-0 min-h-[354px] items-stretch px-6 [&_h2]:text-lg [&>p]:min-h-[2.8em] [&>p]:m-0 [&>p]:text-muted-foreground [&>p]:text-sm [&>p]:leading-relaxed max-[1100px]:p-3 max-[760px]:min-h-0",
            h2 { "Try a demo" }
            p { "Explore a sample keyboard to see BoardStudio in action." }
            Collapsible { class: "relative w-full mt-auto", open: open(), on_open_change: move |next| open.set(next), keep_mounted: true,
                CollapsibleTrigger { r#as: move |attrs: Vec<Attribute>| rsx! {
                    Button { attributes: attrs, variant: ButtonVariant::Default, class: "w-full", aria_expanded: open(), aria_controls: trigger_id.clone(), "Open demo" }
                } }
                CollapsibleContent { id: Some(content_id), class: "grid gap-2 mt-2 [&[hidden]]:hidden", aria_label: "Demo keyboards", hidden: !open(),
                    for demo in DEMO_PROJECTS {
                        {
                            let doc = demo.document(runtime.core_services().open_scratch);
                            let rt = runtime.clone();
                            rsx! {
                                article { class: "grid grid-cols-[72px_minmax(0,1fr)] items-center gap-2 p-2 border border-border rounded-md bg-canvas [&>div:first-child]:row-span-2 [&>div:first-child]:!h-[54px] [&>div:first-child]:!bg-transparent [&>[data-slot=button]]:col-span-full [&>[data-slot=button]]:w-full",
                                    LibraryPreview { preview: preview(&doc), fallback: demo.name.to_owned() }
                                    div { class: "min-w-0", h3 { class: "text-xs font-semibold", "{demo.name}" } p { class: "text-[11px] text-muted-foreground", "{demo.summary}" } }
                                    Button { variant: ButtonVariant::Default, onclick: move |_| {
                                        match rt.open_demo_copy(doc.clone()) {
                                            Ok(()) => { super::close_project_menu(); if let Some(mut b) = browser { b.0.set(false); } }
                                            Err(err) => rt.report(err),
                                        }
                                    }, "Open demo copy" }
                                }
                            }
                        }
                    }
                    Callout { "These examples demonstrate layout only. Add and verify hardware before fabrication." }
                }
            }
        }
    }
}

fn start_new_keyboard(
    runtime: Rc<Runtime>,
    browser: Option<ProjectBrowserState>,
    mut pending_new: Signal<Option<PendingNewKeyboard>>,
    mut new_error: Signal<String>,
) {
    super::close_project_menu();
    if let Some(mut b) = browser {
        b.0.set(false);
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

#[component]
pub fn Library(
    project_menu: bool,
    #[props(default)] menu_page: Option<Signal<super::ProjectMenuPage>>,
) -> Element {
    let _ = (project_menu, menu_page);
    let runtime = use_context::<Rc<Runtime>>();
    let browser = try_use_context::<ProjectBrowserState>();
    let _ = use_context::<Signal<u64>>()();
    let mut guide_request = use_context::<Signal<Option<SetupGuideRequest>>>();
    let pending_new = use_context::<Signal<Option<PendingNewKeyboard>>>();
    let new_error = use_context::<Signal<String>>();
    let mut guide_counter = use_signal(|| 0_u64);
    let mut selected_project = use_signal(|| None::<Arc<ProjectDoc>>);
    let mut rename_open = use_signal(|| false);
    let mut demo_open = use_signal(|| false);
    let mut name_failure = use_signal(|| None::<String>);
    let mut saved = use_signal(Vec::<Arc<ProjectDoc>>::new);
    let mut unreadable = use_signal(Vec::<UnreadableProject>::new);
    let mut search_query = use_signal(String::new);
    let mut pending_delete = use_signal(|| None::<Arc<ProjectDoc>>);
    let mut deleting = use_signal(|| false);
    let mut delete_error = use_signal(String::new);
    let mut pending_unreadable = use_signal(|| None::<UnreadableProject>);
    let mut deleting_unreadable = use_signal(|| false);
    let mut unreadable_err = use_signal(String::new);
    use_context_provider(|| pending_delete);
    let mut status = use_signal(|| ListStatus::Loading);
    let mut retry = use_signal(|| 0_u64);
    let retry_value = retry();
    let generation = use_hook(|| Rc::new(Cell::new(0_u64)));
    let mounted = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let m = mounted.clone();
        move || m.set(false)
    });

    let current_snapshot = runtime.model().accepted;
    let current = current_snapshot.as_ref().map(|s| s.document.clone());
    let current_id = current.as_ref().map(|d| d.id.clone());
    let name_owner = current_snapshot.as_ref().map(ProjectNameOwner::from);
    let tool_request = try_use_context::<ProjectToolRequest>();
    use_effect({
        let (current, rt) = (current.clone(), runtime.clone());
        move || {
            let Some(mut req) = tool_request else {
                return;
            };
            let Some(tool) = req.0() else {
                return;
            };
            req.0.set(None);
            match tool {
                ProjectTool::Rename(id) => {
                    if let Some(cur) = &current
                        && cur.id == id
                        && rt
                            .model()
                            .accepted
                            .as_ref()
                            .is_some_and(|s| s.document.id == id)
                    {
                        selected_project.set(Some(cur.clone()));
                        rename_open.set(true);
                    }
                }
                ProjectTool::Demo => demo_open.set(true),
            }
        }
    });
    let list_id = current_id.clone();
    let (list_rt, list_gen, list_m) = (runtime.clone(), generation.clone(), mounted.clone());
    use_effect(use_reactive!(|list_id, retry_value| {
        let _ = (&list_id, retry_value);
        let Some(g) = list_gen.get().checked_add(1) else {
            status.set(ListStatus::Failed);
            return;
        };
        list_gen.set(g);
        status.set(ListStatus::Loading);
        let (rt, m, gens, req_id) = (
            list_rt.clone(),
            list_m.clone(),
            list_gen.clone(),
            list_id.clone(),
        );
        spawn_local(async move {
            let res = rt.store.list_projects().await;
            let latest = rt.model().accepted.as_ref().map(|s| s.document.id.clone());
            if !m.get() || gens.get() != g || latest != req_id {
                return;
            }
            match res {
                Ok(listing) => {
                    unreadable.set(listing.unreadable);
                    let mut docs = listing.documents;
                    sort_saved(&mut docs);
                    saved.set(docs.into_iter().map(Arc::new).collect());
                    status.set(ListStatus::Ready);
                }
                Err(_) => status.set(ListStatus::Failed),
            }
        });
    }));

    let mut cards: Vec<(Arc<ProjectDoc>, bool)> = Vec::new();
    if let Some(cur) = &current {
        cards.push((Arc::clone(cur), true));
        cards.extend(
            saved
                .read()
                .iter()
                .filter(|d| d.id != cur.id)
                .map(|d| (Arc::clone(d), false)),
        );
    } else {
        cards.extend(saved.read().iter().map(|d| (Arc::clone(d), false)));
    }
    let selected_doc = selected_project().and_then(|sel| {
        cards
            .iter()
            .find(|(d, _)| d.id == sel.id)
            .map(|(d, _)| Arc::clone(d))
    });
    let selected_preview = selected_doc.as_ref().and_then(|d| preview(d));
    let saved_count = cards.len();
    let query = search_query();
    cards.retain(|(d, _)| matches_project_search(d, &query));
    let storage_readiness = runtime.storage_readiness();
    let import_disabled = !matches!(storage_readiness, Ok(true));
    let (new_rt, import_rt, del_rt, del_unr_rt, rename_rt) = (
        runtime.clone(),
        runtime.clone(),
        runtime.clone(),
        runtime.clone(),
        runtime.clone(),
    );
    let (del_m, unr_m, rename_m) = (mounted.clone(), mounted.clone(), mounted.clone());
    let (retry_gen, del_gen) = (generation.clone(), generation.clone());
    rsx! {
        section { id: "m1-library-landing-content", class: "grid grid-cols-[minmax(0,2.7fr)_minmax(280px,1fr)] gap-0 w-full h-full min-h-0 bg-card max-[1100px]:grid-cols-[minmax(0,1.8fr)_minmax(260px,1fr)] max-[760px]:flex max-[760px]:h-auto max-[760px]:min-h-full max-[760px]:flex-col", "aria-label": "Your keyboards",
            if let Err(err) = &storage_readiness { p { role: "alert", class: "text-sm text-destructive", "{err}" } }
            div { class: "col-start-1 min-w-0 min-h-0 overflow-auto pt-14 px-[clamp(24px,3vw,48px)] pb-8 bg-canvas max-[760px]:overflow-visible max-[760px]:px-4 max-[760px]:py-6",
                header { class: "mb-3", h1 { class: "text-3xl tracking-tight", "Your keyboards" } p { class: "mt-1 text-muted-foreground text-sm", "Create, customize and manage your keyboard projects." } }
                div { class: "mb-4 text-muted-foreground text-sm", "{saved_count} saved keyboards" }
                div { class: "grid grid-cols-3 items-stretch gap-3 mb-[34px] max-[760px]:grid-cols-1", aria_label: "Start a keyboard project",
                    DemoActionCard { open: demo_open }
                    Card { class: "min-w-0 min-h-[354px] items-stretch px-6 [&_h2]:text-lg [&>p]:min-h-[2.8em] [&>p]:m-0 [&>p]:text-muted-foreground [&>p]:text-sm [&>p]:leading-relaxed max-[1100px]:p-3 max-[760px]:min-h-0",
                        h2 { "New keyboard" }
                        p { "Start from a blank canvas and build your keyboard." }
                        Button { variant: ButtonVariant::Default, class: "w-full mt-auto", button_type: "button", aria_label: "Create new keyboard", disabled: pending_new().is_some(),
                            onclick: move |_| start_new_keyboard(new_rt.clone(), browser, pending_new, new_error), "Create new" }
                    }
                    Card { class: "min-w-0 min-h-[354px] items-stretch px-6 [&_h2]:text-lg [&>p]:min-h-[2.8em] [&>p]:m-0 [&>p]:text-muted-foreground [&>p]:text-sm [&>p]:leading-relaxed max-[1100px]:p-3 max-[760px]:min-h-0",
                        h2 { "Import project" }
                        p { "Bring in a keyboard file from your computer." }
                        div { class: "w-full mt-auto",
                            Button { variant: ButtonVariant::Default, class: "w-full", disabled: import_disabled, onclick: move |_| {
                                if let Some(i) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("library-import-file")).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) { i.click(); }
                            }, "Import project" }
                            Input { id: "library-import-file", hidden: true, class: "hidden", r#type: InputType::File, accept: ".boardstudio", disabled: import_disabled, onchange: move |e: FormEvent| {
                                if let Some(i) = e.data().try_as_web_event().and_then(|ev| ev.target()).and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
                                    && let Some(f) = i.files().and_then(|fs| fs.get(0))
                                {
                                    import_rt.import_file(f);
                                    if let Some(mut b) = browser { b.0.set(false); }
                                    i.set_value("");
                                }
                            } }
                        }
                    }
                }
                div { class: "flex items-baseline justify-between mb-2 pt-[31px] border-t border-border", h2 { class: "text-base", "Your saved keyboards" } }
                if saved_count > 0 {
                    div { class: "flex items-center gap-2 mb-3.5",
                        InputGroup {
                            InputGroupAddon { svg { class: "size-4 fill-none stroke-current", view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M13.5 13.5 18 18M15 8.5a6.5 6.5 0 1 1-13 0 6.5 6.5 0 0 1 13 0Z" } } }
                            InputGroupInput { r#type: InputType::Search, aria_label: "Search saved keyboards", placeholder: "Search your keyboards", value: "{query}", oninput: move |e: FormEvent| search_query.set(e.value()) }
                        }
                        if !query.is_empty() { Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| search_query.set(String::new()), "Clear search" } }
                    }
                }
                if status() == ListStatus::Loading { p { role: "status", class: "flex items-center gap-2", Spinner {} "Loading saved keyboards…" } }
                if status() == ListStatus::Failed {
                    Alert { variant: AlertVariant::Destructive, "Saved keyboards could not be loaded. "
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, onclick: move |_| { if let Some(g) = retry_gen.get().checked_add(1) { retry_gen.set(g); retry += 1; } }, "Try again" }
                    }
                }
                if !cards.is_empty() {
                    div { class: "grid grid-cols-3 gap-3 max-[760px]:grid-cols-2",
                        for (doc, is_cur) in cards {
                            KeyboardCard { key: "{doc.id}", selected: selected_project().as_ref().is_some_and(|s| s.id == doc.id),
                                document: KeyboardCardSource(doc), current: is_cur,
                                on_select: move |doc: Arc<ProjectDoc>| {
                                    selected_project.set(Some(doc));
                                    focus_element_by_selector("#m1-browser-open-keyboard");
                                }
                            }
                        }
                    }
                } else if status() == ListStatus::Ready && query.trim().is_empty() {
                    Empty { class: "grid justify-items-center gap-4 py-[18px] px-5 text-center",
                        EmptyTitle { "No keyboards yet" }
                        EmptyDescription { "Get started by trying a demo, creating a new keyboard or importing a project." }
                    }
                } else if !query.trim().is_empty() {
                    p { class: "my-3.5 text-muted-foreground text-[13px]", "No keyboards match your search." }
                }
                for rec in unreadable() {
                    Alert { variant: AlertVariant::Destructive, class: "max-w-[min(44vw,440px)] m-0 text-xs",
                        strong { "Unreadable saved keyboard: {rec.label}" }
                        p { "{rec.error}" }
                        Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: { let (rt, k) = (runtime.clone(), rec.key.clone()); move |_| rt.download_raw_project(k.clone()) }, "Download raw saved data (JSON only)" }
                        Button { variant: ButtonVariant::Destructive, button_type: "button", onclick: move |_| { unreadable_err.set(String::new()); pending_unreadable.set(Some(rec.clone())); }, "Delete unreadable keyboard…" }
                    }
                }
                if let Some(pid) = current_id.clone() {
                    div { class: "mt-2",
                        Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| {
                            guide_counter += 1;
                            guide_request.set(Some(SetupGuideRequest { project_id: pid.clone(), request_id: format!("{pid}-guide-{}", guide_counter()), start_at_project: false }));
                            if let Some(mut b) = browser { b.0.set(false); }
                        }, "Setup guide" }
                    }
                }
            }
            aside { class: "col-start-2 row-start-1 self-stretch sticky top-0 overflow-auto flex flex-col gap-3 py-12 px-10 border-l border-border bg-selected max-[1100px]:py-7 max-[1100px]:px-6 max-[760px]:static max-[760px]:px-4 max-[760px]:py-6 max-[760px]:border-t max-[760px]:border-l-0", aria_label: "Keyboard details and starting guidance",
                if let Some(doc) = selected_doc {
                    h2 { class: "text-base", "{project_display_name(&doc)}" }
                    if current_id.as_deref() == Some(doc.id.as_str()) {
                        Collapsible { open: rename_open(), on_open_change: move |o| rename_open.set(o), keep_mounted: true,
                            CollapsibleTrigger { r#as: move |attrs: Vec<Attribute>| rsx! { Button { attributes: attrs, variant: ButtonVariant::Secondary, "Rename project" } } }
                            CollapsibleContent { hidden: !rename_open(),
                                TextCommitField {
                                    label: "Project name".to_owned(), aria_label: "Project name", value: doc.name.clone(),
                                    on_commit: {
                                        let (rt, owner, m) = (rename_rt.clone(), name_owner.clone(), rename_m.clone());
                                        move |val: String| {
                                            let Some(owner) = &owner else { return; };
                                            match project_name_resolver(&rt, owner, &m, &val) {
                                                Ok(resolver) => name_failure.set(rt.commit("project-name", resolver).err()),
                                                Err(ProjectNameRejection::OwnerChanged) => rt.report("Project changed while renaming; the project name was not changed."),
                                                Err(_) => {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(msg) = name_failure() { div { id: "library-project-name-error", FieldError { "{msg}" } } }
                    p { class: "m-0 text-foreground text-sm", "{selected_preview.as_ref().map_or(0, |p| p.keys.len())} keys · {doc.boards.len()} board(s) · Revision {doc.revision}" }
                    div { class: "[&>div]:!h-[clamp(180px,18vw,280px)] [&>div]:!bg-card", LibraryPreview { preview: selected_preview, fallback: "Preview unavailable".to_owned() } }
                    div { class: "grid grid-cols-2 gap-2 [&>#m1-browser-open-keyboard]:col-span-full [&>#m1-browser-open-keyboard]:w-full",
                        Button { id: "m1-browser-open-keyboard", variant: ButtonVariant::Default, onclick: {
                            let (rt, id) = (runtime.clone(), doc.id.clone());
                            move |_| { rt.open_saved(id.clone()); if let Some(mut b) = browser { b.0.set(false); } }
                        }, "Open keyboard" }
                        if current_id.as_deref() == Some(doc.id.as_str()) {
                            Button { variant: ButtonVariant::Secondary, onclick: { let rt = runtime.clone(); move |_| rt.export_project_copy() }, "Save project copy" }
                        }
                        Button { variant: ButtonVariant::Destructive, onclick: { let d = doc.clone(); move |_| pending_delete.set(Some(d.clone())) }, "Delete keyboard" }
                    }
                } else {
                    h2 { class: "text-base", "Start here" }
                    p { class: "text-muted-foreground text-sm", "Choose a demo, create a new keyboard, or import a .boardstudio file." }
                }
                p { class: "mt-8 text-muted-foreground text-xs", "Projects are saved in this browser. Download a .boardstudio copy to move or back them up." }
            }
            AlertDialog { id: Some("library-project-delete".to_owned()), open: pending_delete().is_some(),
                on_open_change: move |o: bool| if !o && !deleting() { pending_delete.set(None); delete_error.set(String::new()); },
                AlertDialogTitle { if let Some(p) = pending_delete() { "Delete “{project_display_name(&p)}”?" } else { "Delete keyboard?" } }
                AlertDialogDescription { "This removes the keyboard saved in this browser. This cannot be undone." }
                if !delete_error().is_empty() { Alert { variant: AlertVariant::Destructive, "{delete_error()}" } }
                AlertDialogActions {
                    AlertDialogCancel { attributes: vec![Attribute::new("disabled", deleting(), None, false)], "Cancel" }
                    Button { variant: ButtonVariant::Destructive, button_type: "button", disabled: deleting(), onclick: move |_| {
                        if deleting() { return; }
                        let Some(proj) = pending_delete() else { return; };
                        deleting.set(true); delete_error.set(String::new());
                        let (rt, m, gens) = (del_rt.clone(), del_m.clone(), del_gen.clone());
                        spawn_local(async move {
                            let res = rt.delete_saved_project(proj.id.clone()).await;
                            if !m.get() { return; }
                            match res {
                                Ok(()) => {
                                    let rem = saved.read().iter().filter(|s| s.id != proj.id).cloned().collect();
                                    saved.set(rem);
                                    if selected_project.peek().as_ref().is_some_and(|s| s.id == proj.id) { selected_project.set(None); }
                                    pending_delete.set(None);
                                    if let Some(g) = gens.get().checked_add(1) { gens.set(g); retry += 1; }
                                    focus_element_by_selector("[type=search][aria-label='Search saved keyboards']");
                                }
                                Err(_) => delete_error.set("This keyboard could not be deleted. Try again.".into()),
                            }
                            deleting.set(false);
                        });
                    }, if deleting() { "Deleting…" } else { "Delete keyboard" } }
                }
            }
            AlertDialog { id: Some("library-unreadable-delete".to_owned()), open: pending_unreadable().is_some(),
                on_open_change: move |o: bool| if !o && !deleting_unreadable() { pending_unreadable.set(None); unreadable_err.set(String::new()); },
                AlertDialogTitle { "Delete unreadable keyboard?" }
                AlertDialogDescription { "Permanently delete this unreadable saved keyboard? This cannot be undone." }
                if !unreadable_err().is_empty() { Alert { variant: AlertVariant::Destructive, "{unreadable_err()}" } }
                AlertDialogActions {
                    AlertDialogCancel { attributes: vec![Attribute::new("disabled", deleting_unreadable(), None, false)], "Cancel" }
                    Button { variant: ButtonVariant::Destructive, disabled: deleting_unreadable(), onclick: move |_| {
                        if deleting_unreadable() { return; }
                        let Some(rec) = pending_unreadable() else { return; };
                        deleting_unreadable.set(true); unreadable_err.set(String::new());
                        let (rt, m) = (del_unr_rt.clone(), unr_m.clone());
                        spawn_local(async move {
                            let res = rt.store.delete_unreadable_by_key(&rec).await;
                            if !m.get() { return; }
                            match res {
                                Ok(()) => { pending_unreadable.set(None); retry += 1; focus_element_by_selector("[type=search][aria-label='Search saved keyboards']"); }
                                Err(e) => unreadable_err.set(format!("The unreadable keyboard was kept: {e}")),
                            }
                            deleting_unreadable.set(false);
                        });
                    }, if deleting_unreadable() { "Deleting…" } else { "Delete unreadable keyboard" } }
                }
            }
            if pending_new().is_some() { p { role: "status", class: "flex items-center gap-2", Spinner {} "Creating keyboard…" } }
            if !new_error().is_empty() { Alert { variant: AlertVariant::Destructive, "{new_error()}" } }
        }
    }
}
