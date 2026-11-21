use super::demo_projects::DEMO_PROJECTS;
use super::setup_guide::{PendingNewKeyboard, SetupGuideRequest};
use crate::project_name::{
    ProjectNameKey, ProjectNameOwner, ProjectNameRejection, project_name_resolver,
};
use crate::runtime::Runtime;
#[cfg(test)]
use boardstudio_application::Event;
#[cfg(test)]
use boardstudio_model::model::{EditCommand, EditOperation, EditPhase};
use boardstudio_model::model::{PartKind, ProjectDoc};
#[cfg(test)]
use boardstudio_web_host::host::ProjectListing;
use boardstudio_web_host::host::UnreadableProject;
use boardstudio_web_ui_shared::draft_edits::{DraftEdits, FieldView};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::alert_dialog::{
    AlertDialog, AlertDialogActions, AlertDialogCancel, AlertDialogDescription, AlertDialogTitle,
};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonSize, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::Callout;
use boardstudio_web_ui_shared::ui::card::Card;
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use boardstudio_web_ui_shared::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use boardstudio_web_ui_shared::ui::field::FieldError;
use boardstudio_web_ui_shared::ui::input::InputType;
use boardstudio_web_ui_shared::ui::input_group::{InputGroup, InputGroupAddon, InputGroupInput};
use boardstudio_web_ui_shared::ui::label::Label;
use boardstudio_web_ui_shared::ui::spinner::Spinner;
use boardstudio_web_ui_shared::ui::tooltip::{Tooltip, TooltipContent, TooltipTrigger};
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

#[derive(Clone)]
struct ProjectNameCommitAction {
    runtime: Rc<Runtime>,
    owner: ProjectNameOwner,
    mounted: Rc<Cell<bool>>,
    edits: DraftEdits<ProjectNameKey>,
    name: FieldView,
    /// The project the latest submitted rename belongs to; its observation is live only
    /// while that project stays accepted.
    submitted_owner: Rc<std::cell::RefCell<Option<ProjectNameOwner>>>,
}

impl ProjectNameCommitAction {
    fn commit(&self, value: &str) {
        match project_name_resolver(&self.runtime, &self.owner, &self.mounted, value) {
            Ok(resolver) => {
                self.edits.submit(
                    &self.runtime,
                    ProjectNameKey::Name,
                    "project-name",
                    Some("project name".into()),
                    resolver,
                    self.name.submitted(value.to_owned()),
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
                    .query_selector("[type=search][aria-label='Search saved keyboards']")
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
    let delete_label = format!("Delete {name}");
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
        Card { class: if selected { "relative min-w-0 p-0 gap-0 bg-selected text-selected-foreground" } else { "relative min-w-0 p-0 gap-0" },
          div { "data-project-id": "{project_id}",
            Button { variant: ButtonVariant::Ghost,
                class: "flex w-full h-auto min-h-11 flex-col items-stretch justify-start gap-[5px] p-3 text-left whitespace-normal",
                button_type: "button",
                aria_label: "{action_name}",
                aria_current: current.then_some("true".to_owned()),
                aria_pressed: on_select.map(|_| selected),
                onclick: move |_| {
                    if let Some(on_select) = on_select { on_select.call(selected_document.clone()); return; }
                    super::close_project_menu();
                    runtime.open_saved(document_id.clone());
                },
                match &preview {
                    Err(()) => rsx! {
                            LibraryPreviewFrame { class: "content-center gap-2 text-muted-foreground text-xs [&_svg]:!w-[30px] [&_svg]:!h-[30px] [&_svg]:justify-self-center",
                                svg { class: "fill-none stroke-current stroke-[1.5] [stroke-linecap:round] [stroke-linejoin:round]", view_box: "0 0 20 20", "aria-hidden": "true",
                                    path { d: "M2 5h16v11H2ZM5 8h.1M8 8h.1M11 8h.1M14 8h.1M5 11h.1M8 11h.1M11 11h.1M14 11h.1M6 14h8" }
                                }
                                span { "Preview unavailable" }
                            }
                    },
                    Ok(preview) if preview.keys.is_empty() => rsx! {
                            LibraryPreviewFrame { class: "content-center gap-2 text-muted-foreground text-xs [&_svg]:!w-[30px] [&_svg]:!h-[30px] [&_svg]:justify-self-center",
                                svg { class: "fill-none stroke-current stroke-[1.5] [stroke-linecap:round] [stroke-linejoin:round]", view_box: "0 0 20 20", "aria-hidden": "true",
                                    path { d: "M2 5h16v11H2ZM5 8h.1M8 8h.1M11 8h.1M14 8h.1M5 11h.1M8 11h.1M11 11h.1M14 11h.1M6 14h8" }
                                }
                                span { "No keys placed" }
                            }
                    },
                    Ok(preview) => rsx! {
                        LibraryPreviewFrame {
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
                span { class: "max-w-full px-1.5 text-foreground text-[13px] font-semibold [overflow-wrap:anywhere] [#m1-library-landing-content_&]:text-lg", "{name}" }
                span { class: "flex flex-wrap items-center gap-x-2 gap-y-1 px-1.5 text-muted-foreground text-[14px]",
                    span { "{detail}" }
                    if current {
                        span { class: "inline-flex items-center gap-0.5 text-primary [&_svg]:size-3",
                            svg { class: "fill-none stroke-current stroke-[1.5] [stroke-linecap:round] [stroke-linejoin:round]", view_box: "0 0 20 20", "aria-hidden": "true", path { d: "m4 10 4 4 8-8" } }
                            "Current"
                            }
                        }
                    },
                }
            }
            if deletable {
                Tooltip { class: "library-card-delete",
                  TooltipTrigger { r#as: move |attributes: Vec<Attribute>| { let delete_document = delete_document.clone(); rsx! {
                    Button { attributes: { let mut attributes = attributes; attributes.push(Attribute::new("data-project-id", document.id.clone(), None, false)); attributes }, variant: ButtonVariant::Ghost, size: ButtonSize::Icon,
                    class: "[&_svg]:size-4 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5] [&_svg]:[stroke-linecap:round] [&_svg]:[stroke-linejoin:round]",
                    button_type: "button",
                    aria_label: delete_label.clone(),
                    aria_haspopup: "dialog", aria_controls: "library-project-delete",
                    aria_expanded: pending_delete().as_ref().is_some_and(|project| project.id == document.id),
                    onclick: move |_| pending_delete.set(Some(delete_document.clone())),
                    svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M3 5h14M7 5V3h6v2M5 5l1 12h8l1-12M8 8v6M12 8v6" } }
                    }
                  } } }
                  TooltipContent { "Delete {name}" }
                }
            }
          }
    }
}

/// Shared frame for project and library previews; callers supply empty-state styling.
#[component]
fn LibraryPreviewFrame(#[props(default)] class: String, children: Element) -> Element {
    rsx! {
        div { class: "grid place-items-center w-full h-[108px] mb-1 p-3.5 rounded bg-canvas text-geometry-key [&_svg]:w-full [&_svg]:h-full [&_svg]:max-w-full [&_rect]:fill-geometry-body [&_rect]:stroke-geometry-key [&_rect]:stroke-[1] max-[760px]:h-[92px] max-[760px]:p-2.5 [#m1-library-landing-content_&]:h-[clamp(120px,12vw,190px)] {class}",
            {children}
        }
    }
}

#[component]
fn LibraryPreview(preview: Option<Preview>, fallback: String) -> Element {
    match preview {
        Some(preview) if preview.keys.is_empty() => rsx! {
            LibraryPreviewFrame { class: "content-center gap-2 text-muted-foreground text-xs [&_svg]:!w-[30px] [&_svg]:!h-[30px] [&_svg]:justify-self-center",
                svg { class: "fill-none stroke-current stroke-[1.5] [stroke-linecap:round] [stroke-linejoin:round]", view_box: "0 0 20 20", "aria-hidden": "true",
                    path { d: "M2 5h16v11H2ZM5 8h.1M8 8h.1M11 8h.1M14 8h.1M5 11h.1M8 11h.1M11 11h.1M14 11h.1M6 14h8" }
                }
                span { "No keys placed" }
            }
        },
        Some(preview) => rsx! {
            LibraryPreviewFrame {
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
            LibraryPreviewFrame { class: "content-center gap-2 text-muted-foreground text-xs [&_svg]:!w-[30px] [&_svg]:!h-[30px] [&_svg]:justify-self-center",
                svg { class: "fill-none stroke-current stroke-[1.5] [stroke-linecap:round] [stroke-linejoin:round]", view_box: "0 0 20 20", "aria-hidden": "true",
                    path { d: "M2 5h16v11H2ZM5 8h.1M8 8h.1M11 8h.1M14 8h.1M5 11h.1M8 11h.1M11 11h.1M14 11h.1M6 14h8" }
                }
                span { "{fallback}" }
            }
        },
    }
}

#[component]
fn DemoActionCard(mut open: Signal<bool>) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let browser = try_use_context::<ProjectBrowserState>();
    let mut summary = use_signal(|| None::<web_sys::HtmlElement>);
    let content_id = format!(
        "library-demo-options-{}",
        dioxus::core::current_scope_id().0
    );
    let trigger_content_id = content_id.clone();
    use_effect(use_reactive((&open,), move |(open,)| {
        if open()
            && let Some(summary) = summary()
        {
            let _ = summary.focus();
        }
    }));
    rsx! {
        Card { class: "min-w-0 min-h-[354px] items-stretch px-6 [&_h2]:text-lg [&>p]:min-h-[2.8em] [&>p]:m-0 [&>p]:text-muted-foreground [&>p]:text-sm [&>p]:leading-relaxed [&>[data-slot=button]]:w-full [&>[data-slot=button]]:mt-auto max-[1100px]:p-3 max-[760px]:min-h-0",
            div { class: "grid h-28 place-items-center text-muted-foreground [&_svg]:w-[min(100%,180px)] [&_svg]:h-full [&_svg]:fill-none [&_svg]:stroke-border-strong [&_svg]:stroke-[1.6] [&_svg]:[stroke-linecap:round] [&_svg]:[stroke-linejoin:round] max-[1100px]:h-[88px] [&_rect:nth-of-type(n+3)]:fill-selected [&_rect:nth-of-type(n+3)]:stroke-geometry-key",
                svg { view_box: "0 0 96 56", "aria-hidden": "true",
                    rect { x: "4", y: "7", width: "88", height: "42", rx: "4" }
                    for (x, y) in [(10, 13), (20, 13), (30, 13), (40, 13), (50, 13), (60, 13), (70, 13), (80, 13), (14, 24), (24, 24), (34, 24), (44, 24), (54, 24), (64, 24), (74, 24), (20, 35), (30, 35), (40, 35), (50, 35), (60, 35), (70, 35)] {
                        rect { x: "{x}", y: "{y}", width: "6", height: "6", rx: "1" }
                    }
                }
            }
            h2 { "Try a demo" }
            p { "Explore a sample keyboard to see BoardStudio in action." }
            Collapsible { class: "relative w-full mt-auto", open: open(), on_open_change: move |next| open.set(next), keep_mounted: true,
                CollapsibleTrigger { r#as: move |attributes: Vec<Attribute>| rsx! {
                    Button { attributes: { let mut attributes = attributes; attributes.push(dioxus::events::onmounted(move |event: MountedEvent| summary.set(event.data().downcast::<web_sys::Element>().and_then(|e| e.dyn_ref::<web_sys::HtmlElement>()).cloned()))); attributes }, variant: ButtonVariant::Default, class: "w-full",
                        aria_expanded: open(), aria_controls: trigger_content_id.clone(),
                        "Open demo" }
                } }
                CollapsibleContent { id: Some(content_id), class: "grid gap-2 mt-2 [&[hidden]]:hidden", aria_label: "Demo keyboards", hidden: !open(), inert: (!open()).then_some(""),
                    for demo in DEMO_PROJECTS {
                        {
                            let document = demo.document(runtime.core_services().open_scratch);
                            let demo_preview = preview(&document).ok();
                            let runtime = runtime.clone();
                            rsx! {
                                article { class: "grid grid-cols-[72px_minmax(0,1fr)] items-center gap-2 p-2 border border-border rounded-md bg-canvas [&>div:first-child]:row-span-2 [&>div:first-child]:!h-[54px] [&>div:first-child]:!bg-transparent [&>[data-slot=button]]:col-span-full [&>[data-slot=button]]:w-full",
                                    LibraryPreview { preview: demo_preview, fallback: demo.name.to_owned() }
                                    div { class: "min-w-0 [&_h3]:text-xs [&_h3]:font-semibold [&_p]:block [&_small]:block [&_p]:mt-0.5 [&_small]:mt-0.5 [&_p]:text-muted-foreground [&_small]:text-muted-foreground [&_p]:text-[11px] [&_small]:text-[11px] [&_p]:leading-[1.35] [&_small]:leading-[1.35]",
                                        h3 { "{demo.name}" }
                                        p { "{demo.summary}" }
                                        small { "Editable layout example" }
                                    }
                                    Button { variant: ButtonVariant::Default, onclick: move |_| {
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
            start_new_keyboard(runtime.clone(), browser, pending_new, new_error);
        }
    });
    let start_new_menu_top = start_new.clone();
    let recovery_required =
        runtime.model().lifecycle == boardstudio_application::Lifecycle::RecoveryRequired;
    let current_snapshot = runtime.model().accepted;
    let current = current_snapshot
        .as_ref()
        .map(|snapshot| snapshot.document.clone());
    let tool_request = try_use_context::<ProjectToolRequest>();
    let mut rename_tool_open = use_signal(|| false);
    let rename_content_id = format!("library-rename-{}", dioxus::core::current_scope_id().0);
    let rename_trigger_content_id = rename_content_id.clone();
    let mut rename_input = use_signal(|| None::<HtmlInputElement>);
    let mut demo_tool_open = use_signal(|| false);
    use_effect(move || {
        if rename_tool_open()
            && let Some(input) = rename_input()
        {
            let _ = input.focus();
        }
    });
    use_effect({
        let current = current.clone();
        let runtime = runtime.clone();
        move || {
            let Some(mut request) = tool_request else {
                return;
            };
            let Some(tool) = request.0() else {
                return;
            };
            request.0.set(None);
            match tool {
                ProjectTool::Rename(id) => {
                    if let Some(current) = &current
                        && current.id == id
                        && runtime
                            .model()
                            .accepted
                            .as_ref()
                            .is_some_and(|s| s.document.id == id)
                    {
                        selected_project.set(Some(current.clone()));
                        rename_tool_open.set(true);
                    }
                }
                ProjectTool::Demo => demo_tool_open.set(true),
            }
        }
    });
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
    let name_edits = use_hook(DraftEdits::<ProjectNameKey>::default);
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
    let mut pending_unreadable = use_signal(|| None::<UnreadableProject>);
    let mut deleting_unreadable = use_signal(|| false);
    let mut unreadable_delete_error = use_signal(String::new);
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
        name: FieldView::new(project_name, name_failure),
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
    let unreadable_delete_runtime = runtime.clone();
    let unreadable_delete_mounted = mounted.clone();
    let is_settings_page = project_menu && menu_page() == super::ProjectMenuPage::Settings;
    let no_saved_projects = cards.is_empty();
    rsx! {
        section { id: if project_menu { "m1-project-menu-dropdown" } else { "m1-library-landing-content" }, class: if project_menu { "fixed z-50 top-[49px] left-3.5 flex w-[min(740px,calc(100vw-28px))] h-[min(760px,calc(100dvh-61px))] max-h-[calc(100dvh-61px)] flex-col gap-0 overflow-hidden p-0 rounded-lg bg-card shadow-lg data-[page=settings]:w-[min(420px,calc(100vw-28px))] data-[page=settings]:h-auto data-[page=settings]:overflow-y-auto max-[760px]:top-12 max-[760px]:left-1.5 max-[760px]:w-[calc(100vw-12px)] max-[760px]:h-[calc(100dvh-54px)] max-[760px]:max-h-[calc(100dvh-54px)] max-[760px]:data-[page=settings]:w-[calc(100vw-12px)] max-[760px]:data-[page=settings]:h-auto" } else { "grid grid-cols-[minmax(0,2.7fr)_minmax(280px,1fr)] gap-0 w-full h-full min-h-0 bg-card max-[1100px]:grid-cols-[minmax(0,1.8fr)_minmax(260px,1fr)] max-[760px]:flex max-[760px]:h-auto max-[760px]:min-h-full max-[760px]:flex-col" }, "aria-label": if project_menu { "Project menu" } else { "Your keyboards" }, "data-page": if project_menu && menu_page() == super::ProjectMenuPage::Settings { "settings" } else { "project" },
            if project_menu {
                header { class: "flex flex-none items-center justify-between gap-3 pt-2.5 pr-4 pb-1.5 pl-5 [&_h2]:m-0 [&_h2]:text-xl [&_h2]:font-semibold [&_svg]:size-[17px] [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5] max-[760px]:pt-1 max-[760px]:pr-2 max-[760px]:pb-0 max-[760px]:pl-3.5",
                    h2 { if menu_page() == super::ProjectMenuPage::Settings { "Workspace settings" } else { "Keyboards" } }
                    Tooltip { TooltipTrigger { r#as: move |attributes: Vec<Attribute>| rsx! {
                        Button { attributes, variant: ButtonVariant::Ghost, size: ButtonSize::Icon, aria_label: "Close project menu", onclick: |_| super::close_project_menu(),
                            svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "m5 5 10 10M15 5 5 15" } }
                        }
                    } } TooltipContent { "Close project menu" } }
                }
            }
            if project_menu && !is_settings_page && browser.is_some() {
                Button { variant: ButtonVariant::Secondary, onclick: move |_| {
                    if let Some(mut browser) = browser { browser.0.set(true); }
                    super::close_project_menu();
                }, "Browse projects" }
            }
            if is_settings_page {
                div { class: "grid content-start gap-3 min-h-0 flex-1 px-5 pb-5 text-foreground",
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        class: "[&_svg]:size-4 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5] [&_svg]:[stroke-linecap:round] [&_svg]:[stroke-linejoin:round]",
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
                p { role: "alert", class: "text-sm text-destructive", "{error}" }
            }
            if project_menu {
                    div { class: "flex flex-none gap-2 pt-0.5 px-5 pb-3.5 [&_svg]:size-[17px] [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5] max-[760px]:pt-0 max-[760px]:px-3.5 max-[760px]:pb-2.5 max-[760px]:[&_button]:min-h-11 [#m1-project-menu-dropdown[data-page=settings]_&]:hidden",
                        Button { variant: ButtonVariant::Secondary, button_type: "button", disabled: pending_new().is_some(), onclick: move |_| start_new_menu_top(),
                            svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M10 3v14M3 10h14" } }
                            "New project"
                        }
                        if has_current {
                            Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| {
                                super::close_project_menu();
                                menu_save_copy.export_project_copy();
                            },
                                svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M10 2v11m-4-4 4 4 4-4M3 14v3h14v-3" } }
                                "Save project copy…"
                            }
                        }
                        div { class: "max-[760px]:min-h-11",
                          Button { variant: ButtonVariant::Secondary, disabled: import_disabled, onclick: move |_| {
                              if let Some(input) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("library-menu-file")).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) { input.click(); }
                          },
                            svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M2 6V4h6l2 2h8v3M2 6v11h14l2-8H5l-3 8" } }
                            "Open project…" }
                            input { id: "library-menu-file", hidden: true, class: "hidden", r#type: "file", accept: ".boardstudio", disabled: import_disabled, onchange: move |event: FormEvent| {
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
                section { class: "flex flex-none flex-col items-stretch gap-1.5 m-0 px-5 py-3 border-y border-border bg-background max-[760px]:px-3.5 max-[760px]:py-2.5 [#m1-project-menu-dropdown[data-page=settings]_&]:hidden", "aria-label": "Current project",
                    Label { class: "flex flex-1 items-center gap-3 w-auto m-0 text-muted-foreground text-xs font-normal [&>span]:flex-none max-[760px]:grid max-[760px]:gap-1",
                        span { "Current project" }
                        InputGroup { InputGroupInput {
                            aria_label: "Project name",
                            aria_invalid: name_failure().is_some(),
                            aria_describedby: name_failure().is_some().then_some("library-project-name-error"),

                            value: "{project_name}",
                            oninput: move |event: FormEvent| { name_failure.set(None); name_dirty.set(true); project_name.set(event.value()); },


                            attributes: vec![dioxus::events::onkeydown({
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
                            }),dioxus::events::onblur(move |_| {
                                if !name_dirty() { return; }
                                name_dirty.set(false);
                                let draft = project_name();
                                if draft.trim().is_empty() {
                                    project_name.set(current_name_for_blur.clone());
                                } else if let Some(action) = rename_action_for_blur.clone() {
                                    action.commit(&draft);
                                }
                            })], } }
                        }
                    if let Some(message) = name_failure() { div { id: "library-project-name-error", FieldError { "{message}" } } }
                    span { class: "text-muted-foreground text-[11px]", "Revision {current_revision} · {current_durability}" }
                    }
                }
            div { class: if project_menu { "min-w-0 min-h-0 flex-1 overflow-y-auto overscroll-contain p-5 [scrollbar-width:thin] max-[760px]:px-3.5 max-[760px]:py-4 [#m1-project-menu-dropdown[data-page=settings]_&]:hidden" } else { "col-start-1 min-w-0 min-h-0 overflow-auto pt-14 px-[clamp(24px,3vw,48px)] pb-8 bg-canvas max-[760px]:overflow-visible max-[760px]:px-4 max-[760px]:py-6" },
                if !project_menu {
                    header { class: "mb-3 [&_h1]:text-3xl [&_h1]:tracking-tight [&_p]:mt-1 [&_p]:text-muted-foreground [&_p]:text-sm",
                        h1 { "Your keyboards" }
                        p { "Create, customize and manage your keyboard projects." }
                    }
                }
                header { class: "flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1 mb-3 [&_h2]:m-0 [&_h2]:text-[15px] [&_h2]:font-semibold [&_h2_span]:ml-[5px] [&_h2_span]:text-muted-foreground [&_h2_span]:text-xs [&_h2_span]:font-normal [&>span]:text-muted-foreground [&>span]:text-xs [#m1-library-landing-content_&]:block [#m1-library-landing-content_&]:mb-4",
                    if project_menu {
                        h2 { "Your keyboards" if status() == ListStatus::Ready { span { "{saved_count}" } } }
                        span { "Saved in this browser" }
                    } else {
                        span { class: "text-muted-foreground !text-sm", "{saved_count} saved keyboards" }
                    }
                }
                if !project_menu {
                    div { class: "grid grid-cols-3 items-stretch gap-3 mb-[34px] max-[760px]:grid-cols-1", aria_label: "Start a keyboard project",
                        DemoActionCard { open: demo_tool_open }
                        Card { class: "min-w-0 min-h-[354px] items-stretch px-6 [&_h2]:text-lg [&>p]:min-h-[2.8em] [&>p]:m-0 [&>p]:text-muted-foreground [&>p]:text-sm [&>p]:leading-relaxed [&>[data-slot=button]]:w-full [&>[data-slot=button]]:mt-auto max-[1100px]:p-3 max-[760px]:min-h-0",
                            div { class: "grid h-28 place-items-center text-muted-foreground [&_svg]:w-[min(100%,180px)] [&_svg]:h-full [&_svg]:fill-none [&_svg]:stroke-border-strong [&_svg]:stroke-[1.6] [&_svg]:[stroke-linecap:round] [&_svg]:[stroke-linejoin:round] max-[1100px]:h-[88px]",
                                svg { view_box: "0 0 96 56", "aria-hidden": "true",
                                    rect { x: "25", y: "5", width: "46", height: "46", rx: "5" }
                                    path { d: "M48 16v24M36 28h24" }
                                }
                            }
                            h2 { "New keyboard" }
                            p { "Start from a blank canvas and build your keyboard." }
                            Button {
                                variant: ButtonVariant::Default,
                                button_type: "button",
                                aria_label: "Create new keyboard",
                                disabled: pending_new().is_some(),
                                onclick: move |_| start_new(),
                                "Create new"
                            }
                        }
                        Card { class: "min-w-0 min-h-[354px] items-stretch px-6 [&_h2]:text-lg [&>p]:min-h-[2.8em] [&>p]:m-0 [&>p]:text-muted-foreground [&>p]:text-sm [&>p]:leading-relaxed [&>[data-slot=button]]:w-full [&>[data-slot=button]]:mt-auto max-[1100px]:p-3 max-[760px]:min-h-0",
                            div { class: "grid h-28 place-items-center text-muted-foreground [&_svg]:w-[min(100%,180px)] [&_svg]:h-full [&_svg]:fill-none [&_svg]:stroke-border-strong [&_svg]:stroke-[1.6] [&_svg]:[stroke-linecap:round] [&_svg]:[stroke-linejoin:round] max-[1100px]:h-[88px]",
                                svg { view_box: "0 0 96 56", "aria-hidden": "true",
                                    path { d: "M25 5h33l14 14v32H25z" }
                                    path { d: "M58 5v14h14M48 41V24m-8 8 8-8 8 8" }
                                }
                            }
                            h2 { "Import project" }
                            p { "Bring in a keyboard file from your computer." }
                            div { class: "w-full mt-auto",
                                Button { variant: ButtonVariant::Default, class: "w-full", disabled: import_disabled, onclick: move |_| {
                                    if let Some(input) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("library-import-file")).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) { input.click(); }
                                }, "Import project" }
                                input { id: "library-import-file", hidden: true, class: "hidden", r#type: "file", accept: ".boardstudio", disabled: import_disabled, onchange: move |event: FormEvent| {
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
                div { class: "flex items-baseline justify-between mb-2 pt-[31px] border-t border-border [&_h2]:text-base [&_span]:text-muted-foreground [&_span]:text-xs",
                    h2 { if project_menu { "Saved keyboards" } else { "Your saved keyboards" } }
                }
                if project_menu || saved_count > 0 {
                div { class: "flex items-center gap-2 mb-3.5 max-[760px]:min-h-11 [&_[data-slot=input-group-addon]_svg]:size-4 [&_[data-slot=input-group-addon]_svg]:fill-none [&_[data-slot=input-group-addon]_svg]:stroke-current",
                    InputGroup {
                     InputGroupAddon { svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M13.5 13.5 18 18M15 8.5a6.5 6.5 0 1 1-13 0 6.5 6.5 0 0 1 13 0Z" } } }
                     InputGroupInput {
                        r#type: InputType::Search,
                        aria_label: "Search saved keyboards",
                        placeholder: "Search your keyboards",
                        value: "{query}",
                        oninput: move |event: FormEvent| search_query.set(event.value()),
                     }
                    }
                    if !query.is_empty() {
                        Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| search_query.set(String::new()), "Clear search" }
                    }
                }
                }
                if status() == ListStatus::Loading {
                    p { role: "status", class: "flex items-center gap-2", Spinner {} "Loading saved keyboards…" }
                }
                if recovery_required && runtime.has_pending_edits() {
                    Alert { variant: AlertVariant::Destructive, class: "max-w-[min(44vw,440px)] m-0 text-xs",
                        p { "Your pending edits have not been saved. Download them before reloading if you want to keep them." }
                        Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: {
                            let runtime = runtime.clone();
                            move |_| runtime.download_pending_edits()
                        }, "Download my pending edits" }
                        Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: {
                            let runtime = runtime.clone();
                            move |_| runtime.recover_saved()
                        }, "Reload saved version" }
                    }
                }
                if status() == ListStatus::Failed {
                    Alert { variant: AlertVariant::Destructive, "Saved keyboards could not be loaded. "
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            class: "max-[760px]:min-h-11",
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
                div { class: "grid grid-cols-3 gap-x-3.5 gap-y-4 max-[760px]:grid-cols-2 max-[760px]:gap-x-2.5 max-[760px]:gap-y-3.5 [#m1-library-landing-content_&]:gap-3",
                    for (document, is_current) in cards {
                        KeyboardCard { key: "{document.id}", selected: selected_project().as_ref().is_some_and(|selected| selected.id == document.id),
                            document: KeyboardCardSource(document), current: is_current, recovery_required, deletable: true,
                            on_select: (!project_menu).then(|| EventHandler::new(move |document: Arc<ProjectDoc>| {
                                let selected_id = document.id.clone();
                                selected_project.set(Some(document));
                                spawn_local(async move {
                                    gloo_timers::future::TimeoutFuture::new(0).await;
                                    let Ok(selected) = selected_project.try_peek() else { return; };
                                    if selected.as_ref().is_none_or(|document| document.id != selected_id)
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
                    Empty { class: "grid justify-items-center gap-4 py-[18px] px-5 text-center [&_svg]:size-14 [&_svg]:fill-none [&_svg]:stroke-muted-foreground [&_svg]:stroke-[1.5] [&_svg]:[stroke-linecap:round] [&_svg]:[stroke-linejoin:round] [&_h2]:text-lg [&_p]:max-w-[54ch] [&_p]:m-0 [&_p]:text-muted-foreground [&_p]:text-sm",
                        svg { view_box: "0 0 48 48", "aria-hidden": "true",
                            path { d: "M5 19h10l4 7h10l4-7h10M5 19l3 20h32l3-20" }
                        }
                        EmptyTitle { "No keyboards yet" }
                        EmptyDescription { "Get started by trying a demo, creating a new keyboard or importing a project." }
                    }
                }
                for record in unreadable() {
                    Alert { variant: AlertVariant::Destructive, class: "max-w-[min(44vw,440px)] m-0 text-xs",
                        strong { "Unreadable saved keyboard: {record.label}" }
                        p { "{record.error}" }
                        Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: {
                            let runtime = runtime.clone();
                            let key = record.key.clone();
                            move |_| runtime.download_raw_project(key.clone())
                        }, "Download raw saved data (JSON only)" }
                        Button { variant: ButtonVariant::Destructive, button_type: "button",
                            aria_haspopup: "dialog", aria_controls: "library-unreadable-delete",
                            aria_expanded: pending_unreadable().as_ref().is_some_and(|pending| pending.key == record.key),
                            onclick: move |_| {
                            unreadable_delete_error.set(String::new());
                            pending_unreadable.set(Some(record.clone()));
                        }, "Delete unreadable keyboard…" }
                    }
                }
                if no_search_matches {
                    p { class: "my-3.5 mx-0 text-muted-foreground text-[13px]", "No keyboards match your search." }
                }
                if !project_menu {
                    div { class: "grid grid-cols-2 gap-2 mt-2 [&_label]:col-span-full max-[760px]:[&_button]:min-h-11",
                        if let Some(project_id) = current_project_id.clone() {
                            Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| {
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
                aside { class: "col-start-2 row-start-1 self-stretch sticky top-0 max-h-none overflow-auto flex flex-col gap-3 py-12 px-10 border-l border-border bg-selected [&>h2]:text-base [&>p]:text-muted-foreground max-[1100px]:py-7 max-[1100px]:px-6 max-[760px]:static max-[760px]:overflow-visible max-[760px]:px-4 max-[760px]:py-6 max-[760px]:border-t max-[760px]:border-l-0 max-[760px]:has-[#m1-browser-open-keyboard]:order-first", aria_label: "Keyboard details and starting guidance",
                    if let Some(document) = selected_document {
                        h2 { "{project_display_name(&document)}" }
                        if current_project_id.as_deref() == Some(document.id.as_str()) {
                            Collapsible { class: "[&_[data-slot=collapsible-content][hidden]]:hidden", open: rename_tool_open(), on_open_change: move |open| rename_tool_open.set(open), keep_mounted: true,
                                CollapsibleTrigger { r#as: move |attributes: Vec<Attribute>| rsx! { Button { attributes, variant: ButtonVariant::Secondary, aria_expanded: rename_tool_open(), aria_controls: rename_trigger_content_id.clone(), "Rename project" } } }
                                CollapsibleContent { id: Some(rename_content_id.clone()), hidden: !rename_tool_open(), inert: (!rename_tool_open()).then_some(""),
                                Label { class: "grid gap-1 text-muted-foreground text-xs",
                                    span { "Project name" }
                                    InputGroup { InputGroupInput {
                                        aria_label: "Project name",
                            aria_invalid: name_failure().is_some(),
                            aria_describedby: name_failure().is_some().then_some("library-project-name-error"),


                                        value: "{project_name}",
                                        oninput: move |event: FormEvent| { name_failure.set(None); name_dirty.set(true); project_name.set(event.value()); },


                                    attributes: vec![dioxus::events::onmounted(move |event: MountedEvent| {
                                            rename_input.set(event.data().downcast::<web_sys::Element>().and_then(|e| e.dyn_ref::<HtmlInputElement>()).cloned());
                                        }),dioxus::events::onkeydown({
                                            let accepted_name = current_name.clone();
                                            move |event: KeyboardEvent| {
                                                match event.data().key().to_string().as_str() {
                                                    "Enter" => {
                                                        event.prevent_default();
                                                        if let Some(input) = event.data().try_as_web_event().and_then(|event| event.target()).and_then(|target| target.dyn_into::<HtmlInputElement>().ok()) { let _ = input.blur(); }
                                                    }
                                                    "Escape" => { name_dirty.set(false); project_name.set(accepted_name.clone()); rename_tool_open.set(false); },
                                                    _ => {}
                                                }
                                            }
                                        }),dioxus::events::onblur(move |_| {
                                            if !name_dirty() { return; }
                                            name_dirty.set(false);
                                            let draft = project_name();
                                            if draft.trim().is_empty() { project_name.set(selection_name_for_blur.clone()); }
                                            else if let Some(action) = selection_rename_action_for_blur.clone() { action.commit(&draft); }
                                        })], } }
                                }
                                }
                            }
                        }
                        if let Some(message) = name_failure() { div { id: "library-project-name-error", FieldError { "{message}" } } }
                        p { class: "m-0 text-foreground text-sm",
                            "{selected_summary}"
                        }
                        p { class: "m-0 text-muted-foreground text-xs", "Saved in this browser" }
                        div { class: "[&>div]:!h-[clamp(180px,18vw,280px)] [&>div]:!bg-card",
                            LibraryPreview { preview: selected_preview, fallback: "Preview unavailable" }
                        }
                        div { class: "grid grid-cols-2 gap-2 [&>#m1-browser-open-keyboard]:col-span-full [&>#m1-browser-open-keyboard]:w-full",
                            Button { id: "m1-browser-open-keyboard", variant: ButtonVariant::Default, onclick: {
                                let runtime = runtime.clone(); let id = document.id.clone();
                                move |_| { runtime.open_saved(id.clone()); if let Some(mut browser) = browser { browser.0.set(false); } }
                            }, if recovery_required { "Recover (discard pending changes)" } else { "Open keyboard" } }
                            if current_project_id.as_deref() == Some(document.id.as_str()) {
                                Button { variant: ButtonVariant::Secondary, onclick: { let runtime = runtime.clone(); move |_| runtime.export_project_copy() }, "Save project copy" }
                            }
                            Button { variant: ButtonVariant::Destructive,
                                aria_haspopup: "dialog", aria_controls: "library-project-delete",
                                aria_expanded: pending_delete().as_ref().is_some_and(|project| project.id == document.id),
                                onclick: {
                                let document = document.clone(); move |_| pending_delete.set(Some(document.clone()))
                            }, "Delete keyboard" }
                        }
                    } else {
                        h2 { "Start here" }
                        p { "Choose an option to get started with BoardStudio." }
                        ol { class: "grid gap-4 my-5 p-0 list-none [&_li]:grid [&_li]:gap-1 [&_strong]:text-foreground [&_strong]:text-sm [&_p]:m-0 [&_p]:text-muted-foreground [&_p]:text-sm [&_p]:leading-relaxed",
                            li { strong { "Try a demo" } p { "Open a sample keyboard to explore features and get familiar with the editor." } }
                            li { strong { "Create a new keyboard" } p { "Start from a blank canvas and bring your ideas to life." } }
                            li { strong { "Import a project" } p { "Load a .boardstudio file from your computer to continue working." } }
                        }
                        p { class: "m-0 pt-3 border-t border-border text-muted-foreground text-sm leading-relaxed", "Demo layouts are starting points; add and verify hardware before fabrication." }
                    }
                    p { class: "mt-8", "Projects are saved in this browser. Download a .boardstudio copy to move or back them up." }
                }
            }
            if project_menu {
                footer { class: "flex flex-none justify-start py-2 px-3.5 bg-background [&_svg]:size-[17px] [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5] max-[760px]:py-1 max-[760px]:px-2 max-[760px]:[&_button]:min-h-11 [#m1-project-menu-dropdown[data-page=settings]_&]:hidden",
                    if let Some(project_id) = current_project_id {
                        Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| {
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
                    Button { variant: ButtonVariant::Secondary, id: "m1-preferences-entry", button_type: "button", onclick: move |_| menu_page.set(super::ProjectMenuPage::Settings),
                        svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M3 5h14M3 10h14M3 15h14M6 3v4M14 8v4M8 13v4" } }
                        "Workspace settings"
                    }
                }
            }
            AlertDialog { id: Some("library-project-delete".to_owned()),
                    open: pending_delete_open,
                    on_open_change: move |open: bool| {
                        if !open && !deleting_project() {
                            pending_delete.set(None);
                            delete_error.set(String::new());
                        }
                    },
                    if let Some(project) = pending_delete() {
                        AlertDialogTitle { "Delete “{project_display_name(&project)}”?" }
                    } else {
                        AlertDialogTitle { "Delete keyboard?" }
                    }
                    AlertDialogDescription { "This removes the keyboard saved in this browser. This cannot be undone. Any project copies you downloaded will be kept." }
                    if !delete_error().is_empty() {
                        Alert { variant: AlertVariant::Destructive, "{delete_error()}" }
                    }
                    AlertDialogActions {
                        AlertDialogCancel { attributes: vec![Attribute::new("disabled", deleting_project(), None, false)], "Cancel" }
                        Button { variant: ButtonVariant::Destructive,
                            button_type: "button",
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
            AlertDialog { id: Some("library-unreadable-delete".to_owned()),
                open: pending_unreadable().is_some(),
                on_open_change: move |open: bool| {
                    if !open && !deleting_unreadable() {
                        pending_unreadable.set(None);
                        unreadable_delete_error.set(String::new());
                    }
                },
                AlertDialogTitle { "Delete unreadable keyboard?" }
                AlertDialogDescription {
                    if let Some(record) = pending_unreadable() { "Permanently delete what we cannot read for saved keyboard {record.label}? " }
                    "Download its raw saved data first if you want to keep it. This cannot be undone."
                }
                if !unreadable_delete_error().is_empty() {
                    Alert { variant: AlertVariant::Destructive, "{unreadable_delete_error()}" }
                }
                AlertDialogActions {
                    AlertDialogCancel { attributes: vec![Attribute::new("disabled", deleting_unreadable(), None, false)], "Cancel" }
                    Button { variant: ButtonVariant::Destructive, disabled: deleting_unreadable(),
                        onclick: move |_| {
                            if deleting_unreadable() { return; }
                            let Some(record) = pending_unreadable() else { return; };
                            deleting_unreadable.set(true);
                            unreadable_delete_error.set(String::new());
                            let runtime = unreadable_delete_runtime.clone();
                            let mounted = unreadable_delete_mounted.clone();
                            spawn_local(async move {
                                let result = runtime.store.delete_unreadable_by_key(&record).await;
                                if !mounted.get() { return; }
                                match result {
                                    Ok(()) => { pending_unreadable.set(None); retry += 1; focus_library_search(); }
                                    Err(error) => unreadable_delete_error.set(format!("The unreadable keyboard was kept: {error}")),
                                }
                                deleting_unreadable.set(false);
                            });
                        },
                        if deleting_unreadable() { "Deleting…" } else { "Delete unreadable keyboard" }
                    }
                }
            }
            if pending_new().is_some() { p { role: "status", class: "flex items-center gap-2", Spinner {} "Creating keyboard…" } }
            if !new_error().is_empty() { Alert { variant: AlertVariant::Destructive, "{new_error()}" } }
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
mod mounted_tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod queued_rename_tests;
