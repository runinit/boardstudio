//! Presentation drafts and DOM input are separate from the durable session state.
mod case_bodies;
mod case_controller;
mod case_display;
mod case_viewer;
mod case_workspace;
mod context_summary;
mod inspector;
mod instance_selection;
mod keycaps_scene;
mod keycaps_workspace;
mod keymap;
mod keymap_workspace;
mod layout_workspace;
mod library;
mod mechanical_settings;
mod mechanical_settings_controller;
mod mechanical_settings_mount;
mod objects;
mod panels;
mod parts;
mod parts_workspace;
mod pcb_workspace;
mod selection;
mod shared_viewer;
mod workspace_composition;

pub(crate) use case_viewer::CaseViewer;
use library::Library;
pub(crate) use mechanical_settings::MechanicalSettings;
pub(crate) use mechanical_settings_mount::MechanicalSettingsMount;
use panels::{InspectorPanel, ObjectsPanel, PanelMode, PanelSide, use_panel_settings};
use parts::{PartsQuery, PartsSelection};
use selection::{ReentrancyReset, SelectionAdapter};
mod footprint_graphics;

use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Durability, Event, Scope, SelectionMode};
use boardstudio_core::model::{
    Contour, EditCommand, EditOperation, EditPhase, Matrix, Part, PartDefinition, Position, Vec2,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use footprint_graphics::FootprintGraphics;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{HtmlElement, SvgElement};

#[derive(Clone)]
struct Drag {
    pointer: i64,
    scope: Scope,
    generation: u64,
    gesture_generation: Option<u64>,
    origin: Vec2,
    client_x: f64,
    client_y: f64,
    positions: Vec<Position>,
    active: bool,
    pan: bool,
    camera: Vec2,
}

#[derive(Clone, Copy)]
struct WorkspaceState(Signal<&'static str>);
/// The explicit UI preference is separate from Session's effective instance.
#[derive(Clone, Copy)]
pub(crate) struct InstanceSelection(Signal<Option<instance_selection::Preference>>);

impl InstanceSelection {
    pub(crate) fn is_current(self, model: &boardstudio_application::ReadModel) -> bool {
        instance_selection::is_current(model, self.0.read().as_ref())
    }
}

#[derive(Clone, Copy)]
struct ThemeState(Signal<&'static str>);
#[derive(Clone, Copy)]
struct ResolvedTheme(Memo<&'static str>);
#[derive(Clone, Copy)]
struct LayerVisibility {
    hidden: Signal<BTreeSet<String>>,
    footprints: Signal<bool>,
}

struct PointerLocation {
    world: Vec2,
    x_fraction: f64,
    y_fraction: f64,
}

#[derive(Clone, Copy)]
struct WorkspaceCallbackSlots {
    select_tree: EventHandler<objects::TreeSelectRequest>,
    navigate: EventHandler<(Scope, String, Option<String>)>,
    nudge_tree: EventHandler<objects::TreeNudgeRequest>,
    parts_select: EventHandler<()>,
    toggle_footprints: EventHandler<()>,
    retry_save: EventHandler<()>,
    recover_saved: EventHandler<()>,
    canvas_mount: EventHandler<MountedEvent>,
    canvas_start_pan: EventHandler<PointerEvent>,
    canvas_move_pointer: EventHandler<PointerEvent>,
    canvas_end_pointer: EventHandler<PointerEvent>,
    canvas_cancel_pointer: EventHandler<PointerEvent>,
    canvas_keyboard: EventHandler<KeyboardEvent>,
    canvas_key_up: EventHandler<KeyboardEvent>,
    canvas_wheel: EventHandler<WheelEvent>,
    keymap_select: EventHandler<String>,
    keycaps_select: EventHandler<String>,
    keymap_layer: EventHandler<String>,
    show_configured_board: EventHandler<String>,
}

#[allow(non_snake_case)]
pub fn App() -> Element {
    let runtime = use_hook(Runtime::new);
    let runtime = match runtime {
        Ok(runtime) => runtime,
        Err(error) => return rsx! { main { role: "alert", "Browser startup failed: {error}" } },
    };
    let version = use_signal(|| 0u64);
    let active = use_hook(|| Rc::new(Cell::new(true)));
    let selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
    let anchor_scope = use_signal(|| None::<Scope>);
    let scope_generation = use_signal(|| 0u64);
    let adapter = use_hook({
        move || SelectionAdapter::new(selected_context, anchor_scope, scope_generation)
    });
    let observed_scope = use_hook({
        let runtime = runtime.clone();
        move || Rc::new(RefCell::new(runtime.scope()))
    });
    let reconciling_scope = use_hook(|| Rc::new(Cell::new(false)));
    use_hook({
        let runtime = runtime.clone();
        let active = active.clone();
        let weak_runtime = Rc::downgrade(&runtime);
        let adapter = adapter.clone();
        let observed_scope = observed_scope.clone();
        let reconciling_scope = reconciling_scope.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                if !active.get() {
                    return;
                }
                let Some(runtime) = weak_runtime.upgrade() else {
                    return;
                };
                let Some(_reset) = ReentrancyReset::enter(reconciling_scope.clone()) else {
                    return;
                };
                let mut selected_context = adapter.selected_context;
                let mut anchor_scope = adapter.anchor_scope;
                let mut generation = adapter.generation;
                let next_scope = runtime.scope();
                let previous_scope = {
                    let mut observed = observed_scope.borrow_mut();
                    std::mem::replace(&mut *observed, next_scope.clone())
                };
                if previous_scope != next_scope {
                    selected_context.set(None);
                    anchor_scope.set(None);
                    generation += 1;
                    let cleanup = adapter
                        .cleanup
                        .borrow()
                        .as_ref()
                        .map(|(_, cleanup)| cleanup.clone());
                    if let Some(cleanup) = cleanup {
                        cleanup();
                    }
                }

                let model = runtime.model();
                let invalid_context =
                    adapter
                        .selected_context
                        .read()
                        .as_ref()
                        .is_some_and(|selected| {
                            next_scope.as_ref() != Some(&selected.scope)
                                || !selection::context_is_current(
                                    &model,
                                    &selected.scope,
                                    &selected.context,
                                )
                        });
                if invalid_context {
                    selected_context.set(None);
                    anchor_scope.set(None);
                }

                let eligible = selection::eligible_live_ids(&model);
                let selected_ids: Vec<_> = model
                    .selected_part_ids
                    .iter()
                    .filter(|id| eligible.iter().any(|allowed| allowed == *id))
                    .cloned()
                    .collect();
                if selected_ids != model.selected_part_ids {
                    anchor_scope.set(None);
                    runtime.submit(Event::SelectParts {
                        operation_id: runtime.operation(),
                        part_ids: selected_ids,
                        range_part_ids: Vec::new(),
                        mode: SelectionMode::Replace,
                    });
                }
                if let Some(scope) = next_scope.as_ref() {
                    let current = runtime.model();
                    if (adapter.anchor_scope)().as_ref() == Some(scope)
                        && current.selection_anchor_id.as_ref().is_none_or(|anchor| {
                            !selection::eligible_live_ids(&current)
                                .iter()
                                .any(|id| id == anchor)
                        })
                    {
                        anchor_scope.set(None);
                    }
                }
                let mut signal = version;
                signal += 1;
            }))
        }
    });
    use_drop({
        let runtime = runtime.clone();
        move || {
            active.set(false);
            runtime.unsubscribe();
            runtime.submit(Event::Close {
                operation_id: runtime.operation(),
            });
        }
    });
    let _ = version();
    use_context_provider(|| runtime.clone());
    use_context_provider(|| version);
    use_context_provider(|| adapter.clone());
    let mut workspace = use_signal(|| "Layout");
    use_context_provider(|| WorkspaceState(workspace));
    let layer_visibility = LayerVisibility {
        hidden: use_signal(BTreeSet::new),
        footprints: use_signal(|| false),
    };
    use_context_provider(|| layer_visibility);
    let theme = use_signal(read_theme_preference);
    let system_theme = use_signal(read_system_theme);
    use_context_provider(|| ThemeState(theme));
    let resolved_theme = use_memo(move || {
        let preference = theme();
        if preference == "system" {
            system_theme()
        } else {
            preference
        }
    });
    use_context_provider(|| ResolvedTheme(resolved_theme));
    let media_listener = use_hook(|| {
        Rc::new(RefCell::new(
            None::<(web_sys::MediaQueryList, Closure<dyn FnMut(web_sys::Event)>)>,
        ))
    });
    use_effect({
        let mut system_theme = system_theme;
        let media_listener = media_listener.clone();
        move || {
            if let Some(query) = web_sys::window().and_then(|window| {
                window
                    .match_media("(prefers-color-scheme: dark)")
                    .ok()
                    .flatten()
            }) {
                let observed_query = query.clone();
                let listener = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                    system_theme.set(if observed_query.matches() {
                        "dark"
                    } else {
                        "light"
                    });
                }) as Box<dyn FnMut(_)>);
                let _ = query
                    .add_event_listener_with_callback("change", listener.as_ref().unchecked_ref());
                *media_listener.borrow_mut() = Some((query, listener));
            }
        }
    });
    use_drop({
        let media_listener = media_listener.clone();
        move || {
            if let Some((query, listener)) = media_listener.borrow_mut().take() {
                let _ = query.remove_event_listener_with_callback(
                    "change",
                    listener.as_ref().unchecked_ref(),
                );
            }
        }
    });
    use_effect(use_reactive((&resolved_theme(),), {
        move |(effective,)| {
            if let Some(root) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.document_element())
            {
                let _ = root.set_attribute("data-theme", effective);
            }
        }
    }));
    use_effect(|| {
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            document.set_title("BoardStudio");
        }
    });
    let project_name = runtime
        .model()
        .accepted
        .as_ref()
        .map(|snapshot| snapshot.document.name.clone())
        .unwrap_or_else(|| "Open project".into());
    let save_label = runtime
        .model()
        .accepted
        .as_ref()
        .map(|_| durability_label(&runtime.model().durability))
        .unwrap_or("Ready");
    let save_state = if runtime.model().accepted.is_none() {
        "ready"
    } else {
        durability_state(&runtime.model().durability)
    };
    rsx! {
        link { rel: "stylesheet", href: "assets/m1.css" }
        main { class: "m1-workbench",
            header { class: "m1-topbar",
                h1 { class: "m1-brand", title: "BoardStudio",
                    svg { view_box: "0 0 30 30", fill: "none", stroke: "currentColor", stroke_width: "1.5", "aria-hidden": "true", path { d: "M4 4h22v22H4zM8 20l5-10 4 8 3-5 3 7" }, circle { cx: "13", cy: "10", r: "1.3" } }
                    span { class: "m1-visually-hidden", "BoardStudio" }
                }
                details { class: "m1-project-menu", onkeydown: move |event: KeyboardEvent| {
                    if event.data().key().to_string() == "Escape" {
                        event.prevent_default();
                        close_project_menu();
                    }
                },
                    summary { "{project_name}" }
                    Library {}
                }
                span { class: "m1-save-state", "data-state": "{save_state}", "{save_label}" }
                WorkspaceNavigation {}
                button { class: "m1-export-tab", id: "m1-tab-Export", "aria-pressed": "{workspace() == \"Export\"}", onclick: move |_| workspace.set("Export"), "Export" }
                ThemePicker {}
            }
            if runtime.model().accepted.is_some() { Editor {} }
            else { LibraryLanding {} }
            p { role: "status", "aria-live": "polite", class: "m1-status", "{runtime.status()}" }
        }
    }
}

fn read_theme_preference() -> &'static str {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item("boardstudio:v2:theme").ok().flatten())
        .filter(|value| value == "light" || value == "dark")
        .map(|value| if value == "dark" { "dark" } else { "light" })
        .unwrap_or("system")
}

fn close_project_menu() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(menu) = document
        .query_selector("details.m1-project-menu")
        .ok()
        .flatten()
    else {
        return;
    };
    let _ = menu.remove_attribute("open");
    if let Some(summary) = menu
        .query_selector("summary")
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
    {
        let _ = summary.focus();
    }
}

fn read_system_theme() -> &'static str {
    if web_sys::window()
        .and_then(|window| {
            window
                .match_media("(prefers-color-scheme: dark)")
                .ok()
                .flatten()
        })
        .is_some_and(|query| query.matches())
    {
        "dark"
    } else {
        "light"
    }
}

#[component]
fn ThemePicker() -> Element {
    let mut theme = use_context::<ThemeState>().0;
    rsx! { label { class: "m1-theme-picker", "Theme"
        select { "aria-label": "Theme", value: "{theme()}", onchange: move |event: FormEvent| {
            let preference = match event.value().as_str() { "light" => "light", "dark" => "dark", _ => "system" };
            if let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten()) {
                let _ = storage.set_item("boardstudio:v2:theme", preference);
            }
            theme.set(preference);
        },
            option { value: "system", "System" }
            option { value: "light", "Light" }
            option { value: "dark", "Dark" }
        }
    } }
}

#[component]
fn CanvasLayers() -> Element {
    let layers = use_context::<LayerVisibility>();
    let mut open = use_signal(|| false);
    let toggle = move |id: &'static str| {
        if id == "Footprints" {
            let mut footprints = layers.footprints;
            footprints.set(!footprints());
        } else {
            let mut hidden = layers.hidden;
            let mut next = (hidden)();
            if !next.insert(id.to_owned()) {
                next.remove(id);
            }
            hidden.set(next);
        }
    };
    let keydown = move |event: KeyboardEvent| {
        if event.data().key().to_string() == "Escape" && open() {
            event.prevent_default();
            open.set(false);
            if let Some(trigger) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id("m1-layers-trigger"))
                .and_then(|element| element.dyn_into::<HtmlElement>().ok())
            {
                let _ = trigger.focus();
            }
        }
    };
    let hidden = (layers.hidden)();
    let entries = ["Keys", "Components", "Keycaps", "Footprints", "Board"];
    rsx! {
        section { class: "m1-layers", "data-open": "{open()}", aria_label: "Canvas layers", onkeydown: keydown,
            button { id: "m1-layers-trigger", class: "m1-layers-trigger", aria_expanded: "{open()}", aria_controls: "m1-layers-list", onclick: move |_| open.set(!open()),
                "Layers"
                svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: if open() { "m5 12 5-5 5 5" } else { "m5 8 5 5 5-5" } } }
            }
            if open() {
                button { class: "m1-layers-close", onclick: move |_| { open.set(false); if let Some(trigger) = web_sys::window().and_then(|window| window.document()).and_then(|document| document.get_element_by_id("m1-layers-trigger")).and_then(|element| element.dyn_into::<HtmlElement>().ok()) { let _ = trigger.focus(); } }, "Close" }
            }
            div { id: "m1-layers-list", class: "m1-layer-list", hidden: !open(),
                for id in entries {
                    { let visible = if id == "Footprints" { (layers.footprints)() } else { !hidden.contains(id) }; let name = id; let label = format!("{} {name}", if visible { "Hide" } else { "Show" });
                        rsx! { button { key: "{id}", aria_pressed: "{visible}", aria_label: "{label}", onclick: move |_| toggle(name),
                            span { class: "m1-layer-swatch", "data-layer": "{id}" }
                            span { class: "m1-layer-label", "{id}" }
                            svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M2 10q8-12 16 0-8 12-16 0Z" }, circle { cx: "10", cy: "10", r: "2.5" }, if !visible { path { d: "m3 17 14-14" } } }
                        } }
                    }
                }
            }
        }
    }
}

#[component]
fn WorkspaceNavigation() -> Element {
    let mut workspace = use_context::<WorkspaceState>().0;
    let tabs = ["Layout", "PCB", "Keymap", "Keycaps", "Case", "Parts"];
    rsx! {
        div { class: "m1-workflow-navigation",
            nav { class: "m1-workflow-tabs", role: "tablist", "aria-label": "Board workflow",
                for tab in tabs {
                    button { key: "{tab}", role: "tab", id: "m1-tab-{tab}", "aria-selected": "{workspace() == tab}", "aria-controls": "m1-workspace-panel", onclick: move |_| workspace.set(tab), onkeydown: {
                        let mut workspace = workspace;
                        move |event: KeyboardEvent| {
                            let key = event.data().key().to_string();
                            let current = workspace();
                            let index = tabs.iter().position(|candidate| *candidate == current).unwrap_or(0);
                            let next = match key.as_str() { "ArrowRight" => Some((index + 1) % tabs.len()), "ArrowLeft" => Some((index + tabs.len() - 1) % tabs.len()), "Home" => Some(0), "End" => Some(tabs.len() - 1), _ => None };
                            if let Some(next) = next {
                                event.prevent_default();
                                workspace.set(tabs[next]);
                                if let Some(tab) = tabs.get(next)
                                    && let Some(element) = web_sys::window().and_then(|window| window.document()).and_then(|document| document.get_element_by_id(&format!("m1-tab-{tab}"))).and_then(|element| element.dyn_into::<HtmlElement>().ok())
                                {
                                    let _ = element.focus();
                                }
                            }
                        }
                    },
                        TabIcon { name: tab }
                        "{tab}"
                    }
                }
            }
            select { class: "m1-workspace-select", "aria-label": "Workspace", value: "{workspace()}", onchange: move |event| workspace.set(match event.value().as_str() { "PCB" => "PCB", "Keymap" => "Keymap", "Keycaps" => "Keycaps", "Case" => "Case", "Parts" => "Parts", "Export" => "Export", _ => "Layout" }),
                for tab in tabs { option { value: "{tab}", "{tab}" } }
                option { value: "Export", "Export" }
            }
        }
    }
}

#[component]
fn TabIcon(name: &'static str) -> Element {
    rsx! { svg { class: "m1-tab-icon", view_box: "0 0 20 20", fill: "none", stroke: "currentColor", stroke_width: "1.5", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
        if name == "Layout" { path { d: "M4 14.5 14.5 4l2.5 2.5L5.5 18H3v-2.5zM11.5 7l2.5 2.5M3 18h14" } }
        else if name == "PCB" { path { d: "M4 4h12v12H4zM7 7h2v2H7zM11 11h2v2h-2zM9 8h3v4" } }
        else if name == "Keymap" { path { d: "M3 5h14v10H3zM6 8h2m2 0h2m2 0h1M6 11h2m2 0h2M6 14h8" } }
        else if name == "Keycaps" { path { d: "m3 15 2-10h10l2 10zM5 5l2 4h6l2-4M7 9l-1 6m7-6 1 6" } }
        else if name == "Case" { path { d: "m10 2 7 4v8l-7 4-7-4V6zM3 6l7 4 7-4m-7 4v8" } }
        else { path { d: "M4 3h9l3 3v11H4zM13 3v4h4M7 11h6M7 14h6" } }
    } }
}

#[component]
fn LibraryLanding() -> Element {
    rsx! {
        section { class: "m1-library-landing",
            h1 { "Open a keyboard" }
            p { "Choose a demo copy, open a saved project, or import a BoardStudio archive." }
            Library {}
        }
    }
}

#[component]
fn ExportPanel() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let model = runtime.model();
    let archive = runtime.clone();
    let step = runtime.clone();
    rsx! {
        section { class: "m1-export-panel", "aria-label": "Export",
            h1 { "Export" }
            p { "Create files from the saved keyboard in the current board and instance scope." }
            div { class: "m1-export-actions",
                button { disabled: model.accepted.is_none(), onclick: move |_| if let Some(scope) = archive.scope() { archive.submit(Event::StartExport { operation_id: archive.operation(), scope }); }, "Export archive" }
                button { disabled: model.accepted.is_none(), onclick: move |_| step.export_step(), "Export STEP" }
            }
            p { role: "status", "aria-live": "polite", "{runtime.status()}" }
        }
    }
}

fn durability_label(durability: &Durability) -> &'static str {
    match durability {
        Durability::Saved { .. } => "Saved",
        Durability::Saving { .. } => "Saving…",
        Durability::Failed { .. } => "Save failed",
        _ => "Pending",
    }
}

fn accepted_board_contours(snapshot: &AcceptedSnapshot, board_id: &str) -> Rc<[Contour]> {
    let contours = snapshot
        .scene
        .board_contours
        .iter()
        .find(|board| board.board_id == board_id)
        .map(|board| board.contours.as_slice())
        .unwrap_or_else(|| {
            if snapshot.document.boards.len() == 1 {
                snapshot.scene.contours.as_slice()
            } else {
                &[]
            }
        });
    Rc::<[Contour]>::from(contours.to_vec())
}

fn keymap_bounds(view: &keymap::KeymapView, contours: &[Contour]) -> Option<(f64, f64, f64, f64)> {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    let mut include = |x: f64, y: f64| {
        bounds = Some(bounds.map_or((x, x, y, y), |(min_x, max_x, min_y, max_y)| {
            (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
        }));
    };
    for key in &view.keys {
        let angle = key.pose.rotation.to_radians();
        let (sin, cos) = angle.sin_cos();
        for (local_x, local_y) in [
            (-key.size.x / 2.0, -key.size.y / 2.0),
            (-key.size.x / 2.0, key.size.y / 2.0),
            (key.size.x / 2.0, -key.size.y / 2.0),
            (key.size.x / 2.0, key.size.y / 2.0),
        ] {
            let x = key.pose.at.x + local_x * cos - local_y * sin;
            let y = key.pose.at.y + local_x * sin + local_y * cos;
            include(x, y);
        }
    }
    for contour in contours {
        for point in &contour.points {
            include(point.x, point.y);
        }
    }
    bounds
}

fn keycaps_bounds(
    view: &keycaps_scene::KeycapsView,
    contours: &[Contour],
) -> Option<(f64, f64, f64, f64)> {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    let mut include = |x: f64, y: f64| {
        bounds = Some(bounds.map_or((x, x, y, y), |(min_x, max_x, min_y, max_y)| {
            (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
        }));
    };
    for key in &view.keys {
        let angle = key.pose.rotation.to_radians();
        let (sin, cos) = angle.sin_cos();
        for (local_x, local_y) in [
            (-key.size.x / 2.0, -key.size.y / 2.0),
            (-key.size.x / 2.0, key.size.y / 2.0),
            (key.size.x / 2.0, -key.size.y / 2.0),
            (key.size.x / 2.0, key.size.y / 2.0),
        ] {
            include(
                key.pose.at.x + local_x * cos - local_y * sin,
                key.pose.at.y + local_x * sin + local_y * cos,
            );
        }
    }
    for contour in contours {
        for point in &contour.points {
            include(point.x, point.y);
        }
    }
    bounds
}

fn keymap_scope_matches(model: &boardstudio_application::ReadModel, scope: &Scope) -> bool {
    model.active_board_id == scope.board_id
        && model.active_instance_id == scope.instance_id
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.session_epoch == scope.session_epoch
                && snapshot.document.id == scope.document_id
        })
}

fn durability_state(durability: &Durability) -> &'static str {
    match durability {
        Durability::Saved { .. } => "saved",
        Durability::Saving { .. } => "saving",
        Durability::Failed { .. } => "failed",
        _ => "pending",
    }
}

#[component]
fn Editor() -> Element {
    let mut workspace_callbacks = use_hook(|| WorkspaceCallbackSlots {
        select_tree: EventHandler::new(|_: objects::TreeSelectRequest| {}),
        navigate: EventHandler::new(|_: (Scope, String, Option<String>)| {}),
        nudge_tree: EventHandler::new(|_: objects::TreeNudgeRequest| {}),
        parts_select: EventHandler::new(|_: ()| {}),
        toggle_footprints: EventHandler::new(|_: ()| {}),
        retry_save: EventHandler::new(|_: ()| {}),
        recover_saved: EventHandler::new(|_: ()| {}),
        canvas_mount: EventHandler::new(|_: MountedEvent| {}),
        canvas_start_pan: EventHandler::new(|_: PointerEvent| {}),
        canvas_move_pointer: EventHandler::new(|_: PointerEvent| {}),
        canvas_end_pointer: EventHandler::new(|_: PointerEvent| {}),
        canvas_cancel_pointer: EventHandler::new(|_: PointerEvent| {}),
        canvas_keyboard: EventHandler::new(|_: KeyboardEvent| {}),
        canvas_key_up: EventHandler::new(|_: KeyboardEvent| {}),
        canvas_wheel: EventHandler::new(|_: WheelEvent| {}),
        keymap_select: EventHandler::new(|_: String| {}),
        keycaps_select: EventHandler::new(|_: String| {}),
        keymap_layer: EventHandler::new(|_: String| {}),
        show_configured_board: EventHandler::new(|_: String| {}),
    });
    let runtime = use_context::<Rc<Runtime>>();
    let adapter = use_context::<SelectionAdapter>();
    let version = use_context::<Signal<u64>>();
    let observed_version = version();
    let instance_preference = use_signal(|| {
        runtime.scope().and_then(|scope| {
            scope
                .instance_id
                .map(|explicit_id| instance_selection::Preference {
                    session_epoch: scope.session_epoch,
                    document_id: scope.document_id,
                    explicit_id,
                })
        })
    });
    let instance_selection = use_context_provider(|| InstanceSelection(instance_preference));
    let case_body_selection = use_signal(|| None::<case_viewer::BodySelection>);
    let case_layer_selection = use_signal(|| None::<case_viewer::LayerSelection>);
    let case_display = use_signal(std::collections::BTreeMap::new);
    use_context_provider(|| case_viewer::CaseSelection {
        body: case_body_selection,
        layer: case_layer_selection,
        display: case_display,
    });
    let workspace = use_context::<WorkspaceState>().0;
    let matrix_inspector = objects::use_matrix_inspector(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let layer_visibility = use_context::<LayerVisibility>();
    let parts_query: PartsQuery = use_signal(String::new);
    let parts_selection: PartsSelection = use_signal(|| None);
    let mut keymap_layer_id = use_signal(|| "base".to_owned());
    let active_workspace = workspace();
    let has_inspector = matches!(
        active_workspace,
        "Layout" | "Parts" | "Keymap" | "Keycaps" | "Case"
    );
    let mut objects_open = use_signal(|| false);
    let mut inspect_open = use_signal(|| false);
    let on_parts_select = {
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        move |_| {
            objects_open.set(false);
            inspect_open.set(true);
        }
    };
    let objects_panel_settings = use_panel_settings(PanelSide::Objects);
    let inspector_panel_settings = use_panel_settings(PanelSide::Inspector);
    let objects_preferences = objects_panel_settings();
    let inspector_preferences = inspector_panel_settings();
    let left_track = if objects_preferences.mode == PanelMode::Pinned {
        "min(var(--m1-left-panel-width), calc(45vw - 126px))"
    } else {
        "32px"
    };
    let right_track = if !has_inspector {
        "0px"
    } else if inspector_preferences.mode == PanelMode::Pinned {
        "min(var(--m1-right-panel-width), calc(55vw - 154px))"
    } else {
        "32px"
    };
    let left_width = objects_preferences
        .width
        .map(|width| format!("--m1-left-panel-width:{width}px;"))
        .unwrap_or_default();
    let right_width = inspector_preferences
        .width
        .map(|width| format!("--m1-right-panel-width:{width}px;"))
        .unwrap_or_default();
    let panel_layout_style = format!(
        "--m1-left-track:{left_track};--m1-right-track:{right_track};{left_width}{right_width}"
    );
    let model = runtime.model();
    let current_scope = runtime.scope();
    let accepted_token = model.accepted.as_ref().map(|snapshot| snapshot.token);
    let active_board_id = model.active_board_id.clone();
    let layer_source =
        current_scope
            .clone()
            .zip(model.accepted.as_ref())
            .map(|(scope, snapshot)| keymap::LayerSource {
                scope,
                token: snapshot.token,
                revision: snapshot.document.revision,
            });
    // Keep the operation observer alive even when the workspace panel is hidden.
    let layer_actions = keymap::use_layer_operations(
        runtime.clone(),
        layer_source.clone(),
        keymap_layer_id,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
    );
    let keymap_layer_value = keymap_layer_id();
    let keymap_projection = use_memo(use_reactive(
        (
            &accepted_token,
            &current_scope,
            &active_board_id,
            &keymap_layer_value,
        ),
        {
            let runtime = runtime.clone();
            move |(token, scope, board_id, layer_id)| {
                let model = runtime.model();
                let snapshot = model.accepted.as_ref()?;
                if token.as_ref() != Some(&snapshot.token) {
                    return None;
                }
                let view = keymap::project(
                    snapshot,
                    scope.as_ref(),
                    board_id.as_ref(),
                    layer_id.as_ref(),
                )?;
                Some((view, accepted_board_contours(snapshot, &board_id)))
            }
        },
    ));
    let keycaps_projection = use_memo(use_reactive(
        (&accepted_token, &current_scope, &active_board_id),
        {
            let runtime = runtime.clone();
            move |(token, scope, board_id)| {
                let model = runtime.model();
                let snapshot = model.accepted.as_ref()?;
                if token.as_ref() != Some(&snapshot.token) {
                    return None;
                }
                let view = keycaps_scene::project(snapshot, scope.as_ref()?, &board_id)?;
                Some((view, accepted_board_contours(snapshot, &board_id)))
            }
        },
    ));
    let encoder_input_actions = keymap::use_encoder_inputs(runtime.clone(), layer_source.clone());
    let binding_actions = keymap::use_binding_operations(
        runtime.clone(),
        keymap::BindingProjectionSources {
            source: layer_source.clone(),
            view: keymap_projection().as_ref().map(|(view, _)| view.clone()),
            encoder_projection: encoder_input_actions.projection,
            current_encoder_projection: encoder_input_actions.current,
        },
        keymap_layer_id,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
    );
    let keymap_projection = keymap_projection.read().clone();
    let keymap_view = keymap_projection.as_ref().map(|(view, _)| view.clone());
    let keymap_contours = keymap_projection.map(|(_, contours)| contours);
    let keycaps_projection = keycaps_projection.read().clone();
    let keycaps_view = keycaps_projection.as_ref().map(|(view, _)| view.clone());
    let keycaps_contours = keycaps_projection.map(|(_, contours)| contours);
    // Keep macro operation observation alive when another workspace hides the panel.
    let macro_actions = keymap::use_macro_operations(
        runtime.clone(),
        layer_source,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
    );
    let on_show_mechanical_board = {
        let runtime = runtime.clone();
        let captured_scope = current_scope.clone();
        let captured_generation = (adapter.generation)();
        let adapter = adapter.clone();
        EventHandler::new(move |board_id: String| {
            let Some(scope) = captured_scope.as_ref() else {
                return;
            };
            if runtime.scope().as_ref() != Some(scope)
                || (adapter.generation)() != captured_generation
            {
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if !snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == board_id)
            {
                return;
            }
            let instance_id = instance_selection::resolve(
                &snapshot.document,
                snapshot.session_epoch,
                &board_id,
                instance_preference.read().as_ref(),
            )
            .map(str::to_owned);
            if scope.board_id == board_id && scope.instance_id == instance_id {
                return;
            }
            let cleanup = adapter
                .cleanup
                .borrow()
                .as_ref()
                .map(|(_, cleanup)| cleanup.clone());
            if let Some(cleanup) = cleanup {
                cleanup();
            }
            let mut selected_context = adapter.selected_context;
            let mut anchor_scope = adapter.anchor_scope;
            selected_context.set(None);
            anchor_scope.set(None);
            runtime.submit(Event::Navigate {
                operation_id: runtime.operation(),
                board_id,
                instance_id,
            });
        })
    };
    let on_show_mechanical_finding = {
        let runtime = runtime.clone();
        let captured_scope = current_scope.clone();
        let captured_token = model.accepted.as_ref().map(|snapshot| snapshot.token);
        let captured_generation = (adapter.generation)();
        let generation = adapter.generation;
        let mut workspace = workspace;
        let mut inspect_open = inspect_open;
        let mut case_selection = case_viewer::CaseSelection {
            body: case_body_selection,
            layer: case_layer_selection,
            display: case_display,
        };
        EventHandler::new(move |finding_id: String| {
            let Some(scope) = captured_scope.as_ref() else {
                return;
            };
            let model = runtime.model();
            if generation() != captured_generation
                || runtime.scope().as_ref() != Some(scope)
                || !instance_selection.is_current(&model)
                || model
                    .accepted
                    .as_ref()
                    .is_none_or(|current| Some(current.token) != captured_token)
            {
                return;
            }
            let Some(scene) = runtime
                .cad_scene()
                .filter(|scene| &scene.scope == scope && Some(scene.token) == captured_token)
            else {
                return;
            };
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let captured_document = &snapshot.document;
            let Some(assembly) = scene.mechanical.as_ref() else {
                return;
            };
            let Some(finding) = assembly
                .diagnostics
                .iter()
                .find(|finding| finding.id == finding_id)
            else {
                return;
            };
            if let Some(layer) = assembly
                .stack
                .iter()
                .find(|layer| finding.target_ids.contains(&layer.id))
            {
                case_selection.select_layer(scope.clone(), layer.id.clone());
                workspace.set("Case");
            } else if let Some(body) = captured_document.case_bodies.iter().find(|body| {
                body.board_id == scope.board_id && finding.target_ids.contains(&body.id)
            }) {
                case_selection.body.set(Some(case_viewer::BodySelection {
                    scope: scope.clone(),
                    body_id: body.id.clone(),
                }));
                workspace.set("Case");
            } else {
                let Some(board) = captured_document
                    .boards
                    .iter()
                    .find(|board| board.id == scope.board_id)
                else {
                    return;
                };
                let part_ids = finding
                    .target_ids
                    .iter()
                    .filter(|id| board.part_ids.contains(id))
                    .cloned()
                    .collect::<Vec<_>>();
                if !part_ids.is_empty() {
                    runtime.submit(Event::SelectParts {
                        operation_id: runtime.operation(),
                        part_ids,
                        range_part_ids: Vec::new(),
                        mode: SelectionMode::Replace,
                    });
                    workspace.set("Layout");
                }
            }
            inspect_open.set(true);
            runtime.report(finding.message.clone());
        })
    };
    let mechanical_settings = mechanical_settings_mount::use_mechanical_settings_mount(
        runtime.clone(),
        adapter.generation,
        workspace,
        instance_selection,
        case_viewer::CaseSelection {
            body: case_body_selection,
            layer: case_layer_selection,
            display: case_display,
        },
        on_show_mechanical_finding,
        on_show_mechanical_board,
    );
    let Some(render_scope) = current_scope.clone() else {
        return rsx! {};
    };
    let render_generation = (adapter.generation)();
    let zoom_percent = model.camera.zoom * 100.0;
    let Some(snapshot) = model.accepted.as_ref() else {
        return rsx! {};
    };
    let document = snapshot.document.clone();
    let scene = model
        .display_preview
        .clone()
        .unwrap_or_else(|| snapshot.scene.clone());
    let board = document
        .boards
        .iter()
        .find(|b| b.id == model.active_board_id);
    let visible: Vec<_> = document
        .parts
        .iter()
        .filter(|p| board.is_some_and(|b| b.part_ids.contains(&p.id)))
        .cloned()
        .collect();
    let definitions: std::collections::BTreeMap<_, _> = document
        .definitions
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect();
    let matrices: Vec<_> = document
        .matrices
        .iter()
        .filter(|matrix| {
            matrix
                .board_id
                .as_deref()
                .is_some_and(|id| id == model.active_board_id)
                || (matrix.board_id.is_none()
                    && (matrix
                        .part_ids
                        .iter()
                        .any(|id| board.is_some_and(|board| board.part_ids.contains(id)))
                        || (matrix.part_ids.is_empty() && document.boards.len() <= 1)))
        })
        .collect();
    let matrix_scenes: std::collections::BTreeMap<_, _> = scene
        .matrix_scenes
        .iter()
        .map(|matrix| (matrix.matrix_id.as_str(), matrix))
        .collect();
    let mut matrix_members = BTreeSet::new();
    for matrix in &matrices {
        if let Some(projected) = matrix_scenes.get(matrix.id.as_str()) {
            matrix_members.extend(
                projected
                    .cells
                    .iter()
                    .filter(|cell| cell.enabled)
                    .filter_map(|cell| cell.member_id.as_deref()),
            );
        }
    }
    let visible_ids: Rc<Vec<String>> =
        Rc::new(visible.iter().map(|part| part.id.clone()).collect());
    let select_tree = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let generation = render_generation;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        move |request: objects::TreeSelectRequest| {
            if (adapter.generation)() != generation
                || runtime.scope().as_ref() != Some(&request.scope)
                || !selection::context_is_current(
                    &runtime.model(),
                    &request.scope,
                    &request.context,
                )
            {
                return;
            }
            let (close_objects, open_inspect) = match &request.context {
                objects::TreeContext::Board { .. } => (false, false),
                objects::TreeContext::LayoutGroup { .. } => (true, false),
                objects::TreeContext::Matrix { .. }
                | objects::TreeContext::Row { .. }
                | objects::TreeContext::Column { .. }
                | objects::TreeContext::Key { .. }
                | objects::TreeContext::Component { .. } => (true, true),
            };
            let scope = request.scope.clone();
            selection::submit_context(&runtime, &adapter, request);
            if (adapter.generation)() != generation || runtime.scope().as_ref() != Some(&scope) {
                return;
            }
            if close_objects {
                objects_open.set(false);
            }
            if open_inspect {
                inspect_open.set(true);
            }
        }
    };
    let on_keymap_select = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let scope = render_scope.clone();
        let generation = render_generation;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        move |key_id: String| {
            if workspace() != "Keymap"
                || runtime.scope().as_ref() != Some(&scope)
                || (adapter.generation)() != generation
            {
                return;
            }
            let model = runtime.model();
            if !keymap_scope_matches(&model, &scope) {
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if key_id.is_empty() {
                let mut selected_context = adapter.selected_context;
                selected_context.set(None);
                let mut anchor_scope = adapter.anchor_scope;
                anchor_scope.set(None);
                runtime.submit(Event::SelectParts {
                    operation_id: runtime.operation(),
                    part_ids: Vec::new(),
                    range_part_ids: Vec::new(),
                    mode: SelectionMode::Replace,
                });
                return;
            }
            let layer_id = keymap_layer_id();
            let Some(view) =
                keymap::project(snapshot, Some(&scope), &model.active_board_id, &layer_id)
            else {
                return;
            };
            if !view.keys.iter().any(|key| key.id.as_ref() == key_id) {
                return;
            }
            let Some(context) = objects::context_for_part(&model, &key_id) else {
                return;
            };
            if !selection::context_is_current(&model, &scope, &context) {
                return;
            }
            let Some(selected_ids) = selection::submit_canvas_selection(
                &runtime,
                &adapter,
                &scope,
                generation,
                context.clone(),
                SelectionMode::Replace,
                Vec::new(),
            ) else {
                return;
            };
            if selected_ids.is_empty()
                || runtime.scope().as_ref() != Some(&scope)
                || (adapter.generation)() != generation
            {
                return;
            }
            let current = runtime.model();
            if !keymap_scope_matches(&current, &scope)
                || !selection::context_is_current(&current, &scope, &context)
            {
                return;
            }
            let Some(snapshot) = current.accepted.as_ref() else {
                return;
            };
            let layer_id = keymap_layer_id();
            let Some(view) =
                keymap::project(snapshot, Some(&scope), &current.active_board_id, &layer_id)
            else {
                return;
            };
            if !view.keys.iter().any(|key| key.id.as_ref() == key_id) {
                return;
            }
            objects_open.set(false);
            inspect_open.set(true);
        }
    };
    let on_keycaps_select = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let scope = render_scope.clone();
        let generation = render_generation;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        move |key_id: String| {
            if workspace() != "Keycaps"
                || runtime.scope().as_ref() != Some(&scope)
                || (adapter.generation)() != generation
            {
                return;
            }
            let model = runtime.model();
            if !keymap_scope_matches(&model, &scope) || !instance_selection.is_current(&model) {
                return;
            }
            if key_id.is_empty() {
                let mut selected_context = adapter.selected_context;
                selected_context.set(None);
                let mut anchor_scope = adapter.anchor_scope;
                anchor_scope.set(None);
                runtime.submit(Event::SelectParts {
                    operation_id: runtime.operation(),
                    part_ids: Vec::new(),
                    range_part_ids: Vec::new(),
                    mode: SelectionMode::Replace,
                });
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let Some(view) = keycaps_scene::project(snapshot, &scope, &model.active_board_id)
            else {
                return;
            };
            if !view.keys.iter().any(|key| key.id.as_ref() == key_id) {
                return;
            }
            let Some(context) = objects::context_for_part(&model, &key_id) else {
                return;
            };
            if !selection::context_is_current(&model, &scope, &context) {
                return;
            }
            let Some(selected_ids) = selection::submit_canvas_selection(
                &runtime,
                &adapter,
                &scope,
                generation,
                context.clone(),
                SelectionMode::Replace,
                Vec::new(),
            ) else {
                return;
            };
            if selected_ids.is_empty()
                || runtime.scope().as_ref() != Some(&scope)
                || (adapter.generation)() != generation
            {
                return;
            }
            let current = runtime.model();
            if !keymap_scope_matches(&current, &scope)
                || !selection::context_is_current(&current, &scope, &context)
                || !instance_selection.is_current(&current)
            {
                return;
            }
            let Some(snapshot) = current.accepted.as_ref() else {
                return;
            };
            let Some(view) = keycaps_scene::project(snapshot, &scope, &current.active_board_id)
            else {
                return;
            };
            if !view.keys.iter().any(|key| key.id.as_ref() == key_id) {
                return;
            }
            objects_open.set(false);
            inspect_open.set(true);
        }
    };
    let on_keymap_layer = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let scope = render_scope.clone();
        let generation = render_generation;
        move |layer_id: String| {
            if workspace() != "Keymap"
                || runtime.scope().as_ref() != Some(&scope)
                || (adapter.generation)() != generation
            {
                return;
            }
            let model = runtime.model();
            if !keymap_scope_matches(&model, &scope) {
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let exists = snapshot
                .document
                .keymap
                .as_ref()
                .filter(|map| !map.layers.is_empty())
                .map_or(layer_id == "base", |map| {
                    map.layers.iter().any(|layer| layer.id == layer_id)
                });
            if exists {
                keymap_layer_id.set(layer_id);
            }
        }
    };
    let nudge_tree =
        {
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let generation = render_generation;
            let mut inspect_open = inspect_open;
            move |request: objects::TreeNudgeRequest| {
                if runtime.scope().as_ref() != Some(&request.scope)
                    || (adapter.generation)() != generation
                    || !((request.dx == 0 && request.dy.abs() == 1)
                        || (request.dy == 0 && request.dx.abs() == 1))
                {
                    return;
                }
                let model = runtime.model();
                let Some(snapshot) = model.accepted.as_ref() else {
                    return;
                };
                let Some(part) = snapshot
                    .document
                    .parts
                    .iter()
                    .find(|part| part.id == request.part_id)
                else {
                    return;
                };
                if part.locked == Some(true) {
                    return;
                }
                let Some(board) = snapshot
                    .document
                    .boards
                    .iter()
                    .find(|board| board.id == model.active_board_id)
                else {
                    return;
                };
                if !board.part_ids.contains(&part.id) {
                    return;
                }
                let Some(context) = objects::context_for_part(&model, &part.id) else {
                    return;
                };
                if !matches!(
                    context,
                    objects::TreeContext::Key { .. }
                        | objects::TreeContext::Component {
                            matrix_id: None,
                            ..
                        }
                ) {
                    return;
                }
                let selected_ids = if model.selected_part_ids.contains(&part.id) {
                    model.selected_part_ids.clone()
                } else {
                    vec![part.id.clone()]
                };
                let eligible = selection::eligible_live_ids(&model);
                let moving: Vec<_> =
                    selected_ids
                        .iter()
                        .filter(|id| eligible.iter().any(|candidate| candidate == *id))
                        .filter_map(|id| {
                            snapshot.document.parts.iter().find(|candidate| {
                                candidate.id == *id && candidate.locked != Some(true)
                            })
                        })
                        .collect();
                if moving.is_empty() {
                    return;
                }
                let step = if request.large_step { 1.0 } else { 0.1 };
                let positions: Vec<_> = moving
                    .iter()
                    .map(|part| Position {
                        id: part.id.clone(),
                        at: Vec2 {
                            x: part.pose.at.x + f64::from(request.dx) * step,
                            y: part.pose.at.y + f64::from(request.dy) * step,
                        },
                    })
                    .collect();
                let moving_ids: Vec<_> = moving.iter().map(|part| part.id.clone()).collect();
                let mut selected_context = adapter.selected_context;
                selected_context.set(Some(objects::ScopedTreeContext {
                    scope: request.scope.clone(),
                    context,
                }));
                let mut anchor_scope = adapter.anchor_scope;
                anchor_scope.set(None);
                runtime.submit(Event::SelectParts {
                    operation_id: runtime.operation(),
                    part_ids: moving_ids.clone(),
                    range_part_ids: Vec::new(),
                    mode: SelectionMode::Replace,
                });
                let current = runtime.model();
                if runtime.scope().as_ref() != Some(&request.scope) {
                    return;
                }
                if current.selection_anchor_id.as_ref() == moving_ids.first()
                    && current.selection_anchor_id.as_ref().is_some_and(|anchor| {
                        selection::eligible_live_ids(&current).contains(anchor)
                    })
                {
                    anchor_scope.set(Some(request.scope));
                }
                inspect_open.set(true);
                let operation_id = runtime.operation();
                runtime.submit(Event::Edit {
                    operation_id,
                    command: EditCommand {
                        base_revision: snapshot.document.revision,
                        transaction_id: format!("tree-nudge-{}", operation_id.0),
                        phase: EditPhase::Commit,
                        target_ids: moving_ids,
                        operation: EditOperation::MoveParts { positions },
                    },
                });
                let direction = match (request.dx, request.dy) {
                    (-1, 0) => "left",
                    (1, 0) => "right",
                    (0, 1) => "up",
                    (0, -1) => "down",
                    _ => return,
                };
                runtime.report(format!(
                    "{} moved {direction} {step:.1} mm. Position X {:.1}, Y {:.1} mm.",
                    part.reference,
                    part.pose.at.x + f64::from(request.dx) * step,
                    part.pose.at.y + f64::from(request.dy) * step,
                ));
            }
        };
    let workspace_rect_bounds = match active_workspace {
        "Keymap" => keymap_view
            .as_deref()
            .and_then(|view| keymap_bounds(view, keymap_contours.as_deref().unwrap_or(&[]))),
        "Keycaps" => keycaps_view
            .as_deref()
            .and_then(|view| keycaps_bounds(view, keycaps_contours.as_deref().unwrap_or(&[]))),
        _ => None,
    };
    let bounds = workspace_rect_bounds.or_else(|| {
        let mut points = visible.iter().map(|part| part.pose.at);
        let first = points.next()?;
        Some(points.fold(
            (first.x, first.x, first.y, first.y),
            |(min_x, max_x, min_y, max_y), point| {
                (
                    min_x.min(point.x),
                    max_x.max(point.x),
                    min_y.min(point.y),
                    max_y.max(point.y),
                )
            },
        ))
    });
    let (min_x, max_x, min_y, max_y) = bounds.unwrap_or((-50.0, 50.0, -50.0, 50.0));
    let min_x = min_x - 20.0;
    let max_x = max_x + 20.0;
    let min_y = min_y - 20.0;
    let max_y = max_y + 20.0;
    let width = (max_x - min_x).max(50.0) / model.camera.zoom;
    let height = (max_y - min_y).max(50.0) / model.camera.zoom;
    let view_x = (min_x + max_x - width) * 0.5 + model.camera.center.x;
    let view_y = -(min_y + max_y + height) * 0.5 - model.camera.center.y;
    let view_box = format!("{view_x} {view_y} {width} {height}");
    let svg = use_hook(|| Rc::new(RefCell::new(None::<SvgElement>)));
    let drag = use_hook(|| Rc::new(RefCell::new(None::<Drag>)));
    let space_down = use_hook(|| Rc::new(Cell::new(false)));
    let interaction_version = use_signal(|| 0_u64);
    let observed_interaction_version = interaction_version();
    let navigate_scoped = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let drag = drag.clone();
        let svg = svg.clone();
        let generation = render_generation;
        let mut selected_context = adapter.selected_context;
        let mut anchor_scope = adapter.anchor_scope;
        move |(captured_scope, board_id, instance_id): (Scope, String, Option<String>)| {
            if runtime.scope().as_ref() != Some(&captured_scope)
                || (adapter.generation)() != generation
            {
                return;
            }
            let model = runtime.model();
            let valid_board = model.accepted.as_ref().is_some_and(|snapshot| {
                snapshot
                    .document
                    .boards
                    .iter()
                    .any(|board| board.id == board_id)
                    && instance_id.as_ref().is_none_or(|instance_id| {
                        snapshot.document.hardware.as_ref().is_some_and(|hardware| {
                            hardware.instances.iter().any(|instance| {
                                instance.id == *instance_id && instance.board_id == board_id
                            })
                        })
                    })
            });
            if !valid_board {
                return;
            }
            if captured_scope.board_id == board_id && captured_scope.instance_id == instance_id {
                return;
            }
            selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&captured_scope));
            selected_context.set(None);
            anchor_scope.set(None);
            runtime.submit(Event::Navigate {
                operation_id: runtime.operation(),
                board_id,
                instance_id,
            });
        }
    };
    let navigate = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let mut navigate_scoped = navigate_scoped.clone();
        let generation = render_generation;
        move |(captured_scope, board_id, requested_instance): (Scope, String, Option<String>)| {
            if runtime.scope().as_ref() != Some(&captured_scope)
                || (adapter.generation)() != generation
            {
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if !snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == board_id)
            {
                return;
            }
            if let Some(id) = requested_instance {
                let valid = snapshot.document.hardware.as_ref().is_some_and(|hardware| {
                    hardware
                        .instances
                        .iter()
                        .any(|instance| instance.id == id && instance.board_id == board_id)
                });
                if !valid {
                    return;
                }
                let mut preference = instance_preference;
                preference.set(Some(instance_selection::Preference {
                    session_epoch: snapshot.session_epoch,
                    document_id: snapshot.document.id.clone(),
                    explicit_id: id,
                }));
            }
            let instance_id = instance_selection::resolve(
                &snapshot.document,
                snapshot.session_epoch,
                &board_id,
                instance_preference.read().as_ref(),
            )
            .map(str::to_owned);
            navigate_scoped((captured_scope, board_id, instance_id));
        }
    };
    let preference_version = instance_preference();
    use_effect(use_reactive(
        (
            &observed_version,
            &preference_version,
            &observed_interaction_version,
        ),
        {
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let scope = render_scope.clone();
            let token = snapshot.token;
            let revision = snapshot.document.revision;
            let generation = render_generation;
            let drag = drag.clone();
            let mut navigate_scoped = navigate_scoped.clone();
            move |_| {
                let model = runtime.model();
                if runtime.scope().as_ref() != Some(&scope)
                    || (adapter.generation)() != generation
                    || !instance_selection::can_reconcile(&model)
                    || drag.borrow().is_some()
                {
                    return;
                }
                let Some(snapshot) = model.accepted.as_ref() else {
                    return;
                };
                if snapshot.token != token || snapshot.document.revision != revision {
                    return;
                }
                let instance_id = instance_selection::resolve(
                    &snapshot.document,
                    snapshot.session_epoch,
                    &scope.board_id,
                    instance_preference.read().as_ref(),
                )
                .map(str::to_owned);
                if instance_id != scope.instance_id {
                    navigate_scoped((scope.clone(), scope.board_id.clone(), instance_id));
                }
            }
        },
    ));
    let instance_scope_pending = !instance_selection.is_current(&model);
    let on_show_configured_board = {
        let mut navigate = navigate.clone();
        let scope = render_scope.clone();
        move |board_id: String| navigate((scope.clone(), board_id, None))
    };
    let cleanup_registration = use_hook({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let drag = drag.clone();
        let svg = svg.clone();
        let space_down = space_down.clone();
        move || {
            let id = adapter.next_cleanup_id.get();
            adapter.next_cleanup_id.set(id.wrapping_add(1).max(1));
            let weak_runtime = Rc::downgrade(&runtime);
            let weak_drag = Rc::downgrade(&drag);
            let weak_svg = Rc::downgrade(&svg);
            let weak_space = Rc::downgrade(&space_down);
            let cleanup: selection::Cleanup = Rc::new(move || {
                let (Some(runtime), Some(drag), Some(svg), Some(space_down)) = (
                    weak_runtime.upgrade(),
                    weak_drag.upgrade(),
                    weak_svg.upgrade(),
                    weak_space.upgrade(),
                ) else {
                    return;
                };
                space_down.set(false);
                let pending = drag.borrow_mut().take();
                if let Some(pending) = pending {
                    if let Some(element) = svg.borrow().as_ref() {
                        let _ = element.release_pointer_capture(pending.pointer as i32);
                    }
                    if pending.active && !pending.pan {
                        selection::cancel_drag_if_owned(&runtime, &pending);
                    }
                }
            });
            *adapter.cleanup.borrow_mut() = Some((id, cleanup));
            id
        }
    });
    use_drop({
        let adapter = adapter.clone();
        move || {
            let mut registered = adapter.cleanup.borrow_mut();
            if registered
                .as_ref()
                .is_some_and(|(id, _)| id == &cleanup_registration)
            {
                registered.take();
            }
        }
    });
    let selected_tree_context = adapter.selected_context.read().clone().filter(|selected| {
        selected.scope == render_scope
            && selection::context_is_current(&model, &selected.scope, &selected.context)
    });
    let show_position_inspector = selected_tree_context.as_ref().is_none_or(|selected| {
        let resolved = selection::resolve_context(&model, &selected.context);
        resolved.is_some_and(|ids| {
            ids.len() == 1 && model.selected_part_ids.as_slice() == ids.as_slice()
        }) && (matches!(&selected.context, objects::TreeContext::Key { .. })
            || matches!(&selected.context, objects::TreeContext::Component { .. }))
    });
    let context_summary = selected_tree_context
        .as_ref()
        .and_then(|selected| context_summary::summarize(&model, &selected.context));
    let mount = {
        let runtime = runtime.clone();
        let svg = svg.clone();
        move |event: MountedEvent| {
            if let Some(element) = event
                .data()
                .try_as_web_event()
                .and_then(|e| e.dyn_into::<SvgElement>().ok())
            {
                runtime.surface(element.clone());
                *svg.borrow_mut() = Some(element);
            }
        }
    };
    let move_pointer = {
        let runtime = runtime.clone();
        let svg = svg.clone();
        let drag = drag.clone();
        let adapter = adapter.clone();
        let render_scope = render_scope.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            let Some(mut current) = drag
                .borrow()
                .clone()
                .filter(|d| d.pointer == i64::from(pointer.pointer_id()))
            else {
                return;
            };
            if current.scope != render_scope || current.generation != render_generation {
                return;
            }
            if runtime.scope().as_ref() != Some(&current.scope)
                || (adapter.generation)() != current.generation
            {
                if let Some(element) = svg.borrow().as_ref() {
                    let _ = element.release_pointer_capture(pointer.pointer_id());
                }
                let mut stored = drag.borrow_mut();
                if stored.as_ref().is_some_and(|stored| {
                    stored.pointer == current.pointer
                        && stored.scope == current.scope
                        && stored.generation == current.generation
                }) {
                    stored.take();
                }
                return;
            }
            if current.active
                && !current.pan
                && !selection::owns_session_gesture(&runtime, &current)
            {
                if let Some(element) = svg.borrow().as_ref() {
                    let _ = element.release_pointer_capture(pointer.pointer_id());
                }
                drag.borrow_mut().take();
                return;
            }
            // A pan owns only the camera delta calculated on pointerup. It has no
            // part targets and must never sample a lingering Session gesture that
            // happens to reuse this DOM pointer ID.
            if current.pan {
                return;
            }
            let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height) else {
                return;
            };
            if !current.active {
                let dx = f64::from(pointer.client_x()) - current.client_x;
                let dy = f64::from(pointer.client_y()) - current.client_y;
                if dx * dx + dy * dy < 16.0 {
                    return;
                }
                if runtime.model().gesture.is_some() {
                    drag.borrow_mut().take();
                    if let Some(element) = svg.borrow().as_ref() {
                        let _ = element.release_pointer_capture(pointer.pointer_id());
                    }
                    return;
                }
                let target_ids: Vec<_> = current
                    .positions
                    .iter()
                    .map(|position| position.id.clone())
                    .collect();
                let operation = runtime.operation();
                runtime.submit(Event::GestureBegin {
                    operation_id: operation,
                    pointer_id: current.pointer,
                    target_ids: target_ids.clone(),
                    transaction_id: format!("drag-{}", operation.0),
                    start: current.positions.clone(),
                    pitch: Vec2 { x: 19.05, y: 19.05 },
                    snap_fraction: 0.25,
                    geometry_snap: true,
                    gap: None,
                    alt: pointer.alt_key(),
                });
                let fresh = runtime.model();
                let gesture = fresh.gesture.filter(|gesture| {
                    gesture.pointer_id == current.pointer && gesture.target_ids == target_ids
                });
                if gesture.as_ref().is_none_or(|_| {
                    runtime.scope().as_ref() != Some(&current.scope)
                        || (adapter.generation)() != current.generation
                }) {
                    drag.borrow_mut().take();
                    if let Some(element) = svg.borrow().as_ref() {
                        let _ = element.release_pointer_capture(pointer.pointer_id());
                    }
                    return;
                }
                current.active = true;
                current.gesture_generation = gesture.map(|gesture| gesture.generation);
                *drag.borrow_mut() = Some(current.clone());
            }
            runtime.submit(Event::GestureSample {
                pointer_id: current.pointer,
                positions: moved(&current, point),
                alt: pointer.alt_key(),
            });
        }
    };
    let end_pointer = {
        let runtime = runtime.clone();
        let svg = svg.clone();
        let drag = drag.clone();
        let adapter = adapter.clone();
        let render_scope = render_scope.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            let Some(current) = drag
                .borrow()
                .clone()
                .filter(|d| d.pointer == i64::from(pointer.pointer_id()))
            else {
                return;
            };
            let mut interaction_version = interaction_version;
            interaction_version += 1;
            if current.scope != render_scope || current.generation != render_generation {
                return;
            }
            if runtime.scope().as_ref() != Some(&current.scope)
                || (adapter.generation)() != current.generation
            {
                if let Some(element) = svg.borrow().as_ref() {
                    let _ = element.release_pointer_capture(pointer.pointer_id());
                }
                let mut stored = drag.borrow_mut();
                if stored.as_ref().is_some_and(|stored| {
                    stored.pointer == current.pointer
                        && stored.scope == current.scope
                        && stored.generation == current.generation
                }) {
                    stored.take();
                }
                return;
            }
            if current.active
                && !current.pan
                && !selection::owns_session_gesture(&runtime, &current)
            {
                if let Some(element) = svg.borrow().as_ref() {
                    let _ = element.release_pointer_capture(pointer.pointer_id());
                }
                drag.borrow_mut().take();
                return;
            }
            if current.pan {
                if let Some(element) = svg.borrow().as_ref() {
                    let rect = element.get_bounding_client_rect();
                    if rect.width() > 0.0 && rect.height() > 0.0 {
                        let scale = (rect.width() / width).min(rect.height() / height);
                        if scale > 0.0 {
                            let camera = runtime.model().camera;
                            runtime.submit(Event::SetCamera {
                                operation_id: runtime.operation(),
                                center: Vec2 {
                                    x: current.camera.x
                                        - (f64::from(pointer.client_x()) - current.client_x)
                                            / scale,
                                    y: current.camera.y
                                        + (f64::from(pointer.client_y()) - current.client_y)
                                            / scale,
                                },
                                zoom: camera.zoom,
                            });
                        }
                    }
                }
                drag.borrow_mut().take();
                if let Some(element) = svg.borrow().as_ref() {
                    let _ = element.release_pointer_capture(pointer.pointer_id());
                }
                return;
            }
            if current.active
                && let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height)
            {
                runtime.submit(Event::GestureEnd {
                    pointer_id: current.pointer,
                    final_positions: moved(&current, point),
                    alt: pointer.alt_key(),
                });
            } else if current.active {
                selection::cancel_drag_if_owned(&runtime, &current);
            }
            drag.borrow_mut().take();
            if let Some(element) = svg.borrow().as_ref() {
                let _ = element.release_pointer_capture(pointer.pointer_id());
            }
        }
    };
    let cancel_pointer = {
        let runtime = runtime.clone();
        let svg = svg.clone();
        let drag = drag.clone();
        let render_scope = render_scope.clone();
        move |event: PointerEvent| {
            let pointer_id = event
                .data()
                .try_as_web_event()
                .map(|event| event.pointer_id());
            let Some(current) = drag.borrow().clone().filter(|current| {
                current.scope == render_scope
                    && current.generation == render_generation
                    && pointer_id.is_none_or(|pointer| current.pointer == i64::from(pointer))
            }) else {
                return;
            };
            let mut interaction_version = interaction_version;
            interaction_version += 1;
            drag.borrow_mut().take();
            if let Some(element) = svg.borrow().as_ref() {
                let _ = element.release_pointer_capture(current.pointer as i32);
            }
            if current.scope == render_scope
                && current.generation == render_generation
                && runtime.scope().as_ref() == Some(&current.scope)
                && current.active
                && !current.pan
            {
                selection::cancel_drag_if_owned(&runtime, &current);
            }
        }
    };
    use_effect(use_reactive((&active_workspace,), {
        let runtime = runtime.clone();
        let drag = drag.clone();
        let svg = svg.clone();
        let space_down = space_down.clone();
        let render_scope = render_scope.clone();
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        move |_| {
            space_down.set(false);
            selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&render_scope));
            let mut interaction_version = interaction_version;
            interaction_version += 1;
            if active_workspace == "Keymap" || active_workspace == "Case" {
                objects_open.set(false);
                inspect_open.set(true);
            }
        }
    }));
    let keyboard = {
        let runtime = runtime.clone();
        let drag = drag.clone();
        let svg = svg.clone();
        let render_scope = render_scope.clone();
        let space_down = space_down.clone();
        move |event: KeyboardEvent| {
            let key = event.data().key().to_string();
            let code = event.data().code().to_string();
            let modifiers = event.data().modifiers();
            if key == " " || code == "Space" {
                space_down.set(true);
                event.prevent_default();
            } else if key == "Escape" {
                space_down.set(false);
                selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&render_scope));
                let mut interaction_version = interaction_version;
                interaction_version += 1;
            } else if (modifiers.ctrl() || modifiers.meta()) && key.eq_ignore_ascii_case("z") {
                event.prevent_default();
                runtime.submit(if modifiers.shift() {
                    Event::Redo {
                        operation_id: runtime.operation(),
                    }
                } else {
                    Event::Undo {
                        operation_id: runtime.operation(),
                    }
                });
            }
        }
    };
    let key_up = {
        let space_down = space_down.clone();
        move |event: KeyboardEvent| {
            let key = event.data().key().to_string();
            let code = event.data().code().to_string();
            if key == " " || code == "Space" {
                space_down.set(false);
            }
        }
    };
    let start_pan = {
        let drag = drag.clone();
        let runtime = runtime.clone();
        let svg = svg.clone();
        let space_down = space_down.clone();
        let scope = render_scope.clone();
        let adapter = adapter.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            if !space_down.get()
                || pointer.button() != 0
                || runtime.scope().as_ref() != Some(&scope)
                || (adapter.generation)() != render_generation
                || drag.borrow().is_some()
                || runtime.model().gesture.is_some()
            {
                return;
            }
            pointer.prevent_default();
            pointer.stop_propagation();
            if let Some(surface) = svg.borrow().as_ref() {
                let _ = surface.set_pointer_capture(pointer.pointer_id());
                let options = web_sys::FocusOptions::new();
                options.set_prevent_scroll(true);
                let _ = surface.focus_with_options(&options);
            }
            *drag.borrow_mut() = Some(Drag {
                pointer: i64::from(pointer.pointer_id()),
                scope: scope.clone(),
                generation: render_generation,
                gesture_generation: None,
                origin: Vec2::default(),
                client_x: f64::from(pointer.client_x()),
                client_y: f64::from(pointer.client_y()),
                positions: vec![],
                active: true,
                pan: true,
                camera: runtime.model().camera.center,
            });
        }
    };
    let wheel = {
        let runtime = runtime.clone();
        let svg = svg.clone();
        let scope = render_scope.clone();
        let adapter = adapter.clone();
        move |event: WheelEvent| {
            if runtime.scope().as_ref() != Some(&scope)
                || (adapter.generation)() != render_generation
            {
                return;
            }
            let Some(wheel) = event.data().try_as_web_event() else {
                return;
            };
            wheel.prevent_default();
            let surface = svg.borrow();
            let Some(element) = surface.as_ref() else {
                return;
            };
            let rect = element.get_bounding_client_rect();
            if rect.width() <= 0.0 || rect.height() <= 0.0 {
                return;
            }
            let old = runtime.model().camera;
            let base_width = (max_x - min_x).max(50.0);
            let base_height = (max_y - min_y).max(50.0);
            let Some(location) = pointer_location(
                &rect,
                wheel.client_x(),
                wheel.client_y(),
                view_x,
                view_y,
                width,
                height,
            ) else {
                return;
            };
            let world_x = location.world.x;
            let world_y = location.world.y;
            let zoom = (old.zoom * (-wheel.delta_y() * 0.001).exp()).clamp(0.15, 8.0);
            let center = Vec2 {
                x: world_x
                    - (min_x + max_x - base_width / zoom) * 0.5
                    - location.x_fraction * base_width / zoom,
                y: world_y - (min_y + max_y + base_height / zoom) * 0.5
                    + location.y_fraction * base_height / zoom,
            };
            runtime.submit(Event::SetCamera {
                operation_id: runtime.operation(),
                center,
                zoom,
            });
        }
    };
    let undo = runtime.clone();
    let redo = runtime.clone();
    let retry = runtime.clone();
    let recover = runtime.clone();
    workspace_callbacks
        .select_tree
        .replace(Box::new(select_tree));
    workspace_callbacks.navigate.replace(Box::new(navigate));
    workspace_callbacks.nudge_tree.replace(Box::new(nudge_tree));
    workspace_callbacks
        .parts_select
        .replace(Box::new(on_parts_select));
    workspace_callbacks
        .keymap_select
        .replace(Box::new(on_keymap_select.clone()));
    workspace_callbacks
        .keycaps_select
        .replace(Box::new(on_keycaps_select.clone()));
    workspace_callbacks
        .keymap_layer
        .replace(Box::new(on_keymap_layer));
    workspace_callbacks
        .show_configured_board
        .replace(Box::new(on_show_configured_board));
    workspace_callbacks
        .toggle_footprints
        .replace(Box::new(move |_| {
            let mut footprints = layer_visibility.footprints;
            footprints.set(!footprints());
        }));
    {
        let retry = retry.clone();
        workspace_callbacks.retry_save.replace(Box::new(move |_| {
            retry.submit(Event::RetrySave {
                operation_id: retry.operation(),
            });
        }));
    }
    {
        let recover = recover.clone();
        workspace_callbacks
            .recover_saved
            .replace(Box::new(move |_| recover.recover_saved()));
    }
    workspace_callbacks
        .canvas_mount
        .replace(Box::new(mount.clone()));
    workspace_callbacks
        .canvas_start_pan
        .replace(Box::new(start_pan.clone()));
    workspace_callbacks
        .canvas_move_pointer
        .replace(Box::new(move_pointer.clone()));
    workspace_callbacks
        .canvas_end_pointer
        .replace(Box::new(end_pointer.clone()));
    workspace_callbacks
        .canvas_cancel_pointer
        .replace(Box::new(cancel_pointer.clone()));
    workspace_callbacks
        .canvas_keyboard
        .replace(Box::new(keyboard.clone()));
    workspace_callbacks
        .canvas_key_up
        .replace(Box::new(key_up.clone()));
    workspace_callbacks
        .canvas_wheel
        .replace(Box::new(wheel.clone()));
    let shared_objects = workspace_composition::SharedObjectsInput {
        selected_context: adapter.selected_context,
        on_select: workspace_callbacks.select_tree,
        on_navigate: workspace_callbacks.navigate,
        on_nudge: workspace_callbacks.nudge_tree,
    };
    let objects_input = match active_workspace {
        "PCB" => workspace_composition::WorkspaceObjectsInput::Pcb(shared_objects),
        "Keymap" => workspace_composition::WorkspaceObjectsInput::Keymap(shared_objects),
        "Keycaps" => workspace_composition::WorkspaceObjectsInput::Keycaps(shared_objects),
        "Case" => workspace_composition::WorkspaceObjectsInput::Case(shared_objects),
        "Parts" => {
            workspace_composition::WorkspaceObjectsInput::Parts(parts_workspace::ObjectsInput {
                snapshot: snapshot.clone(),
                scope: current_scope.clone(),
                query: parts_query,
                selected: parts_selection,
                on_select: workspace_callbacks.parts_select,
            })
        }
        _ => workspace_composition::WorkspaceObjectsInput::Layout(shared_objects),
    };
    let toolbar_input = match active_workspace {
        "Layout" => {
            let footprints_pressed = (layer_visibility.footprints)()
                && !(layer_visibility.hidden)().contains("Footprints");
            workspace_composition::WorkspaceToolbarInput::Layout(layout_workspace::ToolbarInput {
                selection_indicator: context_summary
                    .as_ref()
                    .map(|summary| summary.indicator.clone()),
                document_name: document.name.clone(),
                save_failure: match &model.durability {
                    Durability::Failed { reason, .. } => Some(reason.clone()),
                    _ => None,
                },
                recovery_required: model.lifecycle
                    == boardstudio_application::Lifecycle::RecoveryRequired,
                footprints_pressed,
                on_toggle_footprints: workspace_callbacks.toggle_footprints,
                on_retry_save: workspace_callbacks.retry_save,
                on_recover_saved: workspace_callbacks.recover_saved,
            })
        }
        "PCB" => workspace_composition::WorkspaceToolbarInput::Pcb,
        "Keymap" => workspace_composition::WorkspaceToolbarInput::Keymap,
        "Keycaps" => workspace_composition::WorkspaceToolbarInput::Keycaps,
        "Case" => workspace_composition::WorkspaceToolbarInput::Case,
        "Parts" => workspace_composition::WorkspaceToolbarInput::Parts,
        "Export" => workspace_composition::WorkspaceToolbarInput::Export,
        _ => workspace_composition::WorkspaceToolbarInput::Other,
    };
    let canvas_handlers = workspace_composition::CanvasEventHandlers {
        mount: workspace_callbacks.canvas_mount,
        start_pan: workspace_callbacks.canvas_start_pan,
        move_pointer: workspace_callbacks.canvas_move_pointer,
        end_pointer: workspace_callbacks.canvas_end_pointer,
        cancel_pointer: workspace_callbacks.canvas_cancel_pointer,
        keyboard: workspace_callbacks.canvas_keyboard,
        key_up: workspace_callbacks.canvas_key_up,
        wheel: workspace_callbacks.canvas_wheel,
    };
    let canvas_input = match active_workspace {
        "PCB" => Some(workspace_composition::WorkspaceCanvasInput::Pcb(
            workspace_composition::PlaceholderInput {
                workspace,
                name: "PCB",
                message: "PCB editing is not available yet in the Rust interface.",
            },
        )),
        "Keymap" => Some(workspace_composition::WorkspaceCanvasInput::Keymap(
            Box::new(keymap_workspace::CanvasInput {
                view: keymap_view.clone(),
                contours: keymap_contours
                    .clone()
                    .unwrap_or_else(|| Rc::<[Contour]>::from(Vec::new())),
                view_box: view_box.clone(),
                selected_ids: model.selected_part_ids.iter().cloned().collect(),
                handlers: canvas_handlers,
                on_select_key: workspace_callbacks.keymap_select,
            }),
        )),
        "Keycaps" => Some(workspace_composition::WorkspaceCanvasInput::Keycaps(
            Box::new(keycaps_workspace::CanvasInput {
                view: keycaps_view.clone(),
                contours: keycaps_contours
                    .clone()
                    .unwrap_or_else(|| Rc::<[Contour]>::from(Vec::new())),
                view_box: view_box.clone(),
                selected_ids: model.selected_part_ids.iter().cloned().collect(),
                handlers: canvas_handlers,
                on_select_key: workspace_callbacks.keycaps_select,
            }),
        )),
        "Case" => Some(workspace_composition::WorkspaceCanvasInput::Case(Box::new(
            case_workspace::CanvasInput {
                mechanical_settings: mechanical_settings.clone(),
                instance_scope_pending,
            },
        ))),
        "Parts" => Some(workspace_composition::WorkspaceCanvasInput::Parts(
            workspace_composition::PlaceholderInput {
                workspace,
                name: "Parts",
                message: "Parts library editing is not available yet in the Rust interface.",
            },
        )),
        "Layout" | "Export" => None,
        _ => Some(workspace_composition::WorkspaceCanvasInput::Other(
            workspace_composition::PlaceholderInput {
                workspace,
                name: active_workspace,
                message: "Parts library editing is not available yet in the Rust interface.",
            },
        )),
    };
    let inspector_input = match active_workspace {
        "Keymap" => workspace_composition::WorkspaceInspectorInput::Keymap(Box::new(
            keymap_workspace::InspectorInput {
                view: keymap_view.clone(),
                scope: render_scope.clone(),
                layer_actions,
                active_layer_id: keymap_layer_id(),
                selected_key_id: model.selected_part_ids.first().cloned(),
                on_layer: workspace_callbacks.keymap_layer,
                on_select_key: workspace_callbacks.keymap_select,
                binding_actions,
                macro_actions,
                admission_token: snapshot.token,
                admission_revision: snapshot.document.revision,
                scope_generation: (adapter.generation)(),
            },
        )),
        "Case" => {
            workspace_composition::WorkspaceInspectorInput::Case(case_workspace::InspectorInput {
                on_show_configured_board: workspace_callbacks.show_configured_board,
                instance_scope_pending,
            })
        }
        "Parts" => {
            workspace_composition::WorkspaceInspectorInput::Parts(parts_workspace::InspectorInput {
                snapshot: snapshot.clone(),
                scope: current_scope.clone(),
                query: parts_query,
                selected: parts_selection,
            })
        }
        "PCB" => workspace_composition::WorkspaceInspectorInput::Pcb,
        "Keycaps" => workspace_composition::WorkspaceInspectorInput::Keycaps(
            keycaps_workspace::InspectorInput {
                view: keycaps_view.clone(),
                selected_key_id: model.selected_part_ids.first().cloned(),
                on_select_key: workspace_callbacks.keycaps_select,
            },
        ),
        _ => workspace_composition::WorkspaceInspectorInput::Layout(
            layout_workspace::InspectorInput {
                context_title: context_summary
                    .as_ref()
                    .map(|summary| summary.title.clone()),
                context_detail: context_summary
                    .as_ref()
                    .and_then(|summary| summary.detail.clone()),
                show_position_inspector,
                matrix_inspector,
            },
        ),
    };
    rsx! {
        section { class: "m1-editor", "aria-label": "Keyboard editor",
            nav { class: "m1-compact-panel-controls", "aria-label": "Panel visibility",
                button { "aria-controls": "m1-objects-panel", "aria-expanded": "{objects_open()}", onclick: move |_| objects_open.set(!objects_open()), "Objects" }
                if has_inspector {
                    button { "aria-controls": "m1-inspector-panel", "aria-expanded": "{inspect_open()}", onclick: move |_| inspect_open.set(!inspect_open()), "Inspect" }
                }
            }
            div { class: "m1-editor-body", style: "{panel_layout_style}",
                ObjectsPanel { compact_open: objects_open, settings: objects_panel_settings,
                    {workspace_composition::objects(objects_input)}
                }
                section { class: "m1-workspace-content", role: "tabpanel", id: "m1-workspace-panel", "aria-labelledby": "m1-tab-{active_workspace}",
                    onfocusin: move |_| {
                        let compact = web_sys::window()
                            .and_then(|window| window.match_media("(max-width: 760px)").ok().flatten())
                            .is_some_and(|query| query.matches());
                        if workspace() == "Case" && compact {
                            // Reveal the focused workspace control without moving focus or remounting the viewer.
                            if objects_open() {
                                objects_open.set(false);
                            }
                            if inspect_open() {
                                inspect_open.set(false);
                            }
                        }
                    },
                    {workspace_composition::toolbar(toolbar_input)}
                    if active_workspace == "Layout" {
                        svg { class: "m1-canvas", view_box: "{view_box}", preserve_aspect_ratio: "xMidYMid meet", tabindex: "0", role: "group", "aria-label": "Keyboard layout; drag components, hold Shift for range selection, hold Space and drag to pan, or use position controls", onmounted: mount,
                    onpointerdown: start_pan, onpointermove: move_pointer, onpointerup: end_pointer, onpointercancel: cancel_pointer.clone(), onlostpointercapture: cancel_pointer, onkeydown: keyboard, onkeyup: key_up, onwheel: wheel,
                    g { transform: "scale(1,-1)",
                        if !(layer_visibility.hidden)().contains("Board") {
                            for contour in scene.board_contours.iter().filter(|b| b.board_id == model.active_board_id).flat_map(|b| &b.contours) {
                                polygon { points: polygon_points(&contour.points), class: if contour.hole { "m1-outline is-hole" } else { "m1-outline" } }
                            }
                        }
                        if !(layer_visibility.hidden)().contains("Keys") {
                            for matrix in &matrices {
                                if let Some(projected) = matrix_scenes.get(matrix.id.as_str()) {
                                    for cell in projected.cells.iter().filter(|cell| cell.enabled) {
                                        {
                                            let member = cell.member_id.as_deref().and_then(|id| visible.iter().find(|part| part.id == id));
                                            let cell_definition = match member {
                                                Some(part) => definitions.get(part.definition_id.as_str()).copied(),
                                                None => definitions.get(matrix.definition_id.as_str()).copied(),
                                            };
                                            let size = resolved_matrix_keycap(member, cell_definition, matrix);
                                            let pose = cell.pose;
                                            let cell_context = objects::TreeContext::Key { matrix_id: matrix.id.clone(), row: cell.row, column: cell.column };
                                            let selected = cell.member_id.as_ref().is_some_and(|id| model.selected_part_ids.contains(id))
                                                || selected_tree_context.as_ref().is_some_and(|selected| selected.context == cell_context);
                                            let matrix_id = matrix.id.clone();
                                            let cell_row = cell.row;
                                            let cell_column = cell.column;
                                            let runtime = runtime.clone();
                                            let adapter = adapter.clone();
                                            let space_down = space_down.clone();
                                            let scope = render_scope.clone();
                                            let generation = render_generation;
                                            let range_ids = visible_ids.clone();
                                            rsx! { rect { class: if selected { "m1-matrix-key is-selected" } else { "m1-matrix-key" }, x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}", rx: "0.9", transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation})", "data-matrix-id": "{matrix.id}", "data-row": "{cell.row}", "data-column": "{cell.column}",
                                                onpointerdown: move |event: PointerEvent| {
                                                    let Some(pointer) = event.data().try_as_web_event() else { return; };
                                                    if pointer.button() != 0 { return; }
                                                    if space_down.get() { return; }
                                                    pointer.prevent_default();
                                                    pointer.stop_propagation();
                                                    let current = runtime.model();
                                                    let Some(context) = objects::context_for_cell(&current, &matrix_id, cell_row, cell_column) else { return; };
                                                    let mode = if pointer.shift_key() { SelectionMode::Range } else if pointer.ctrl_key() || pointer.meta_key() { SelectionMode::Toggle } else { SelectionMode::Replace };
                                                    selection::submit_canvas_selection(&runtime, &adapter, &scope, generation, context, mode, if mode == SelectionMode::Range { range_ids.as_ref().clone() } else { Vec::new() });
                                                }
                                            } }
                                        }
                                    }
                                }
                            }
                        }
                        for part in visible.iter().cloned() {
                            {
                                let pose = scene.transforms.iter().find(|t| t.id == part.id).map(|t| t.pose).unwrap_or(part.pose);
                                let definition = definitions.get(part.definition_id.as_str()).copied();
                                let courtyard = definition.map(|d| polygon_points(&d.courtyard)).unwrap_or_default();
                                let selected = model.selected_part_ids.contains(&part.id);
                                let side_transform = if matches!(part.side, boardstudio_core::model::Side::Back) { "scale(-1 1)" } else { "" };
                                let is_matrix_key = matrix_members.contains(part.id.as_str());
                                let layer_visible = if is_matrix_key { !(layer_visibility.hidden)().contains("Keys") } else { !(layer_visibility.hidden)().contains("Components") };
                                let is_encoder = definition.is_some_and(|definition| matches!(definition.kind, boardstudio_core::model::PartKind::Encoder) || definition.input_profile.as_ref().is_some_and(|profile| profile.rotary.is_some()));
                                let keycap = if is_encoder { None } else {
                                    let standalone_switch = definition.is_some_and(|definition| matches!(definition.kind, boardstudio_core::model::PartKind::Switch));
                                    if is_matrix_key || standalone_switch {
                                        part.keycap.or_else(|| definition.and_then(|definition| definition.keycap)).or_else(|| {
                                            if !is_matrix_key { return None; }
                                            matrices.iter().find_map(|matrix| {
                                                let projected = matrix_scenes.get(matrix.id.as_str())?;
                                                projected.cells.iter().any(|cell| cell.enabled && cell.member_id.as_deref() == Some(part.id.as_str())).then(|| resolved_matrix_keycap(Some(&part), definition, matrix))
                                            })
                                        })
                                    } else { None }
                                };
                                let show_keycap = keycap.filter(|_| !(layer_visibility.hidden)().contains("Keycaps"));
                                let footprints_on = (layer_visibility.footprints)() && !(layer_visibility.hidden)().contains("Footprints");
                                let id = part.id.clone();
                                let runtime = runtime.clone(); let svg = svg.clone(); let drag = drag.clone(); let space_down = space_down.clone();
                                let adapter = adapter.clone(); let render_scope_for_hit = render_scope.clone();
                                let mut selected_context = adapter.selected_context;
                                let generation_for_hit = render_generation;
                                let range_ids = visible_ids.clone();
                                rsx! { if layer_visible { g { key: "{part.id}", class: "m1-scene-part", transform: "translate({pose.at.x},{pose.at.y}) rotate({pose.rotation}) {side_transform}", "data-part-id": "{part.id}",
                                    onpointerdown: move |event: PointerEvent| {
                                        let Some(pointer) = event.data().try_as_web_event() else { return; };
                                        if pointer.button() != 0 { return; }
                                        if runtime.scope().as_ref() != Some(&render_scope_for_hit) || (adapter.generation)() != generation_for_hit { return; }
                                        if drag.borrow().is_some() || runtime.model().gesture.is_some() { return; }
                                        pointer.prevent_default(); pointer.stop_propagation();
                                        if space_down.get() {
                                            if let Some(svg) = svg.borrow().as_ref() { let _ = svg.set_pointer_capture(pointer.pointer_id()); let options = web_sys::FocusOptions::new(); options.set_prevent_scroll(true); let _ = svg.focus_with_options(&options); }
                                            *drag.borrow_mut() = Some(Drag { pointer: i64::from(pointer.pointer_id()), scope: render_scope_for_hit.clone(), generation: generation_for_hit, gesture_generation: None, origin: Vec2::default(), client_x: f64::from(pointer.client_x()), client_y: f64::from(pointer.client_y()), positions: vec![], active: true, pan: true, camera: runtime.model().camera.center });
                                            return;
                                        }
                                        let Some(origin) = coordinates(&svg, &pointer, view_x, view_y, width, height) else {
                                            if let Some(surface) = svg.borrow().as_ref() { let _ = surface.release_pointer_capture(pointer.pointer_id()); }
                                            return;
                                        };
                                        let mode = if pointer.shift_key() { SelectionMode::Range } else if pointer.ctrl_key() || pointer.meta_key() { SelectionMode::Toggle } else { SelectionMode::Replace };
                                        let current = runtime.model();
                                        let Some(context) = objects::context_for_part(&current, &id) else { return; };
                                        if !current.selected_part_ids.contains(&id) || mode != SelectionMode::Replace {
                                            selection::submit_canvas_selection(&runtime, &adapter, &render_scope_for_hit, generation_for_hit, context, mode, if mode == SelectionMode::Range { range_ids.as_ref().clone() } else { Vec::new() });
                                        } else {
                                            let anchor_valid = (adapter.anchor_scope)().as_ref() == Some(&render_scope_for_hit)
                                                && current.selection_anchor_id.as_ref() == Some(&id)
                                                && selection::eligible_live_ids(&current).contains(&id);
                                            if !anchor_valid {
                                                selection::submit_canvas_selection(&runtime, &adapter, &render_scope_for_hit, generation_for_hit, context, SelectionMode::Add, Vec::new());
                                            } else {
                                                selected_context.set(Some(objects::ScopedTreeContext { scope: render_scope_for_hit.clone(), context }));
                                            }
                                        }
                                        let current = runtime.model();
                                        if !current.selected_part_ids.contains(&id) { return; }
                                        let Some(snapshot) = current.accepted else { return; };
                                        let positions: Vec<_> = snapshot.document.parts.iter().filter(|p| current.selected_part_ids.contains(&p.id)).map(|p| Position { id: p.id.clone(), at: p.pose.at }).collect();
                                        if positions.is_empty() { return; }
                                        if let Some(svg) = svg.borrow().as_ref() { let _ = svg.set_pointer_capture(pointer.pointer_id()); let options = web_sys::FocusOptions::new(); options.set_prevent_scroll(true); let _ = svg.focus_with_options(&options); }
                                        *drag.borrow_mut() = Some(Drag { pointer: i64::from(pointer.pointer_id()), scope: render_scope_for_hit.clone(), generation: generation_for_hit, gesture_generation: None, origin, client_x: f64::from(pointer.client_x()), client_y: f64::from(pointer.client_y()), positions, active: false, pan: false, camera: Vec2::default() });
                                    },
                                    polygon { points: "{courtyard}", class: if selected { "m1-part selected" } else { "m1-part" } }
                                    if footprints_on {
                                        if let Some(definition) = definition {
                                            FootprintGraphics { definition: definition.clone(), parameters: part.generator_parameters.clone() }
                                            for pad in &definition.pads {
                                                {
                                                    let rx = match &pad.shape { boardstudio_core::model::PadShape::Circle | boardstudio_core::model::PadShape::Oval => pad.size.x.min(pad.size.y) / 2.0, boardstudio_core::model::PadShape::Roundrect => pad.size.x.min(pad.size.y) / 4.0, boardstudio_core::model::PadShape::Rect => 0.0 };
                                                    rsx! { g { transform: "translate({pad.at.x} {pad.at.y}) rotate({pad.rotation.unwrap_or(0.0)})",
                                                        if pad.plated != Some(false) { rect { class: "m1-part-pad", x: "{-pad.size.x / 2.0}", y: "{-pad.size.y / 2.0}", width: "{pad.size.x}", height: "{pad.size.y}", rx: "{rx}" } }
                                                        if let Some(drill) = pad.drill { circle { class: "m1-part-drill", r: "{drill / 2.0}" } }
                                                    } }
                                                }
                                            }
                                        }
                                    }
                                    if let Some(size) = show_keycap {
                                        { let inset = 1.5_f64.min(size.x / 6.0).min(size.y / 6.0); rsx! { g { class: if selected { "m1-keycap-overlay is-selected" } else { "m1-keycap-overlay" }, "aria-hidden": "true",
                                            rect { x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}", rx: "0.9" }
                                            rect { class: "m1-keycap-top", x: "{-size.x / 2.0 + inset}", y: "{-size.y / 2.0 + inset}", width: "{size.x - inset * 2.0}", height: "{size.y - inset * 2.0}", rx: "0.7" }
                                        } } }
                                    }
                                    if let Some(size) = keycap {
                                        rect { class: "m1-part-hit-area", x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}" }
                                    }
                                    if show_keycap.is_none() { text { transform: "scale(1,-1)", text_anchor: "middle", class: "m1-part-label", x: "0", y: "-5.2", "{part.reference}" } }
                                }} }
                            }
                        }
                    }
                        }
                        CanvasLayers {}
                    } else if active_workspace == "Export" {
                        ExportPanel {}
                    } else if let Some(input) = canvas_input {
                        {workspace_composition::canvas(input)}
                    } else {
                        span {}
                    }
                }
                if has_inspector {
                    InspectorPanel { compact_open: inspect_open, settings: inspector_panel_settings,
                        {workspace_composition::inspector(inspector_input)}
                    }
                }
            }
            footer { class: "m1-editor-footer",
                button { "aria-label": "Undo", onclick: move |_| undo.submit(Event::Undo { operation_id: undo.operation() }), svg { view_box: "0 0 20 20", fill: "none", stroke: "currentColor", stroke_width: "1.5", "aria-hidden": "true", path { d: "M8 6 4 10l4 4M4 10h7a5 5 0 0 1 5 5" } } }
                button { "aria-label": "Redo", onclick: move |_| redo.submit(Event::Redo { operation_id: redo.operation() }), svg { view_box: "0 0 20 20", fill: "none", stroke: "currentColor", stroke_width: "1.5", "aria-hidden": "true", path { d: "m12 6 4 4-4 4m4-4H9a5 5 0 0 0-5 5" } } }
                span { "{document.name} · Revision {document.revision} · {durability_label(&model.durability)}" }
                span { "{zoom_percent:.0}%" }
            }
        }
    }
}

fn resolved_matrix_keycap(
    part: Option<&Part>,
    definition: Option<&PartDefinition>,
    matrix: &Matrix,
) -> Vec2 {
    part.and_then(|part| part.keycap)
        .or_else(|| definition.and_then(|definition| definition.keycap))
        .unwrap_or(Vec2 {
            x: (matrix.pitch.x - matrix.edge_gap.map(|gap| gap.x).unwrap_or(1.0)).max(1.0),
            y: (matrix.pitch.y - matrix.edge_gap.map(|gap| gap.y).unwrap_or(1.0)).max(1.0),
        })
}

fn polygon_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|p| format!("{},{}", p.x, p.y))
        .collect::<Vec<_>>()
        .join(" ")
}
fn coordinates(
    svg: &Rc<RefCell<Option<SvgElement>>>,
    pointer: &web_sys::PointerEvent,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Option<Vec2> {
    let surface = svg.borrow();
    let rect = surface.as_ref()?.get_bounding_client_rect();
    pointer_location(
        &rect,
        pointer.client_x(),
        pointer.client_y(),
        x,
        y,
        width,
        height,
    )
    .map(|location| location.world)
}

fn pointer_location(
    rect: &web_sys::DomRect,
    client_x: i32,
    client_y: i32,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
) -> Option<PointerLocation> {
    if rect.width() <= 0.0 || rect.height() <= 0.0 || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let scale = (rect.width() / width).min(rect.height() / height);
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let content_width = width * scale;
    let content_height = height * scale;
    let left = rect.left() + (rect.width() - content_width) * 0.5;
    let top = rect.top() + (rect.height() - content_height) * 0.5;
    let x_fraction = (f64::from(client_x) - left) / content_width;
    let y_fraction = (f64::from(client_y) - top) / content_height;
    Some(PointerLocation {
        world: Vec2 {
            x: view_x + x_fraction * width,
            y: -(view_y + y_fraction * height),
        },
        x_fraction,
        y_fraction,
    })
}
fn moved(drag: &Drag, point: Vec2) -> Vec<Position> {
    drag.positions
        .iter()
        .map(|p| Position {
            id: p.id.clone(),
            at: Vec2 {
                x: p.at.x + point.x - drag.origin.x,
                y: p.at.y + point.y - drag.origin.y,
            },
        })
        .collect()
}
