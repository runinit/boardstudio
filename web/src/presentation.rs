//! Presentation drafts and DOM input are separate from the durable session state.
mod footprint_graphics;

use crate::{cad_presentation::CasePanel, runtime::Runtime};
use boardstudio_application::{Durability, Event, SelectionMode};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, Position, Vec2};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use footprint_graphics::FootprintGraphics;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlElement, HtmlInputElement, SvgElement};

#[derive(Clone)]
struct Drag {
    pointer: i64,
    origin: Vec2,
    client_x: f64,
    client_y: f64,
    positions: Vec<Position>,
    active: bool,
    pan: bool,
    camera: Vec2,
}

#[derive(Clone)]
struct NumericEdit {
    id: String,
    revision: u64,
    transaction_id: String,
    start: Vec2,
}
type KeyboardHandler = Rc<RefCell<Box<dyn FnMut(KeyboardEvent)>>>;
#[derive(Clone, Copy)]
struct WorkspaceState(Signal<&'static str>);
#[derive(Clone, Copy)]
struct ThemeState(Signal<&'static str>);
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

#[allow(non_snake_case)]
pub fn App() -> Element {
    let runtime = use_hook(Runtime::new);
    let runtime = match runtime {
        Ok(runtime) => runtime,
        Err(error) => return rsx! { main { role: "alert", "Browser startup failed: {error}" } },
    };
    let version = use_signal(|| 0u64);
    let active = use_hook(|| Rc::new(Cell::new(true)));
    use_hook({
        let runtime = runtime.clone();
        let active = active.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                if active.get() {
                    let mut signal = version;
                    signal += 1;
                }
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
    use_effect(use_reactive((&theme(), &system_theme()), {
        move |(preference, system)| {
            let effective = if preference == "system" {
                system
            } else {
                preference
            };
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
fn Objects() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return rsx! {};
    };
    let document = &snapshot.document;
    let items: Rc<Vec<_>> = Rc::new(
        document
            .parts
            .iter()
            .filter(|part| {
                document
                    .boards
                    .iter()
                    .find(|board| board.id == model.active_board_id)
                    .is_some_and(|board| board.part_ids.contains(&part.id))
            })
            .enumerate()
            .map(|(index, part)| {
                let kind = document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == part.definition_id)
                    .map(|definition| format!("{:?}", definition.kind))
                    .unwrap_or_else(|| "component".into());
                (
                    index,
                    part.id.clone(),
                    part.reference.clone(),
                    kind,
                    model.selected_part_ids.contains(&part.id),
                )
            })
            .collect(),
    );
    let selected = items.iter().any(|item| item.4);
    let board_runtime = runtime.clone();
    let instance_runtime = runtime.clone();
    let board_id = model.active_board_id.clone();
    let active_instance = model.active_instance_id.clone().unwrap_or_default();
    let instances: Vec<_> = document
        .hardware
        .as_ref()
        .map(|hardware| {
            hardware
                .instances
                .iter()
                .filter(|instance| instance.board_id == model.active_board_id)
                .collect()
        })
        .unwrap_or_default();
    let select: Rc<dyn Fn(String)> = Rc::new({
        let runtime = runtime.clone();
        move |id| {
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: vec![id],
                range_part_ids: vec![],
                mode: SelectionMode::Replace,
            })
        }
    });
    rsx! {
        aside { class: "m1-objects", "aria-label": "Objects",
            header { h2 { "Objects" } }
            div { class: "m1-object-navigation",
                label { "Board"
                    select { "aria-label": "Board", value: "{model.active_board_id}", onchange: move |event: FormEvent| board_runtime.submit(Event::Navigate { operation_id: board_runtime.operation(), board_id: event.value(), instance_id: None }),
                        for board in &document.boards { option { value: "{board.id}", "{board.name}" } }
                    }
                }
                if !instances.is_empty() {
                    label { "Physical instance"
                        select { "aria-label": "Physical instance", value: "{active_instance}", onchange: move |event: FormEvent| {
                            let value = event.value();
                            instance_runtime.submit(Event::Navigate { operation_id: instance_runtime.operation(), board_id: board_id.clone(), instance_id: (!value.is_empty()).then_some(value) });
                        },
                            option { value: "", "Canonical board" }
                            for instance in &instances { option { key: "{instance.id}", value: "{instance.id}", "{instance.name}" } }
                        }
                    }
                }
            }
            div { class: "m1-object-tree",
                div { class: "m1-object-tree-heading", "{document.name}", span { "{items.len()} parts" } }
                div { role: "listbox", "aria-label": "Objects on current board", class: "m1-component-list",
                    for (index, id, reference, kind, is_selected) in items.iter().cloned() {
                        {
                            let click = select.clone();
                            let key_select = select.clone();
                            let items_for_key = items.clone();
                            rsx! { button { key: "{id}", id: "m1-object-{index}", class: if is_selected { "m1-component selected" } else { "m1-component" }, role: "option", "aria-label": "{reference}, {kind}", "aria-selected": "{is_selected}", tabindex: if is_selected || (!selected && index == 0) { "0" } else { "-1" }, onclick: move |_| click(id.clone()), onkeydown: move |event: KeyboardEvent| {
                                let key = event.data().key().to_string();
                                let next = match key.as_str() { "ArrowDown" => Some(index + 1), "ArrowUp" => Some(index.saturating_sub(1)), "Home" => Some(0), "End" => Some(items_for_key.len().saturating_sub(1)), _ => None };
                                let Some(next) = next.filter(|next| *next < items_for_key.len()) else { return; };
                                event.prevent_default();
                                if let Some((_, next_id, _, _, _)) = items_for_key.get(next) { key_select(next_id.clone()); }
                                if let Some(element) = web_sys::window().and_then(|window| window.document()).and_then(|document| document.get_element_by_id(&format!("m1-object-{next}"))).and_then(|element| element.dyn_into::<HtmlElement>().ok()) { let _ = element.focus(); }
                            }, span { class: "m1-object-reference", "{reference}" } span { class: "m1-object-kind", "{kind}" } } }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PlaceholderWorkspace(name: &'static str) -> Element {
    let mut workspace = use_context::<WorkspaceState>().0;
    let message = match name {
        "PCB" => "PCB editing is not available yet in the Rust interface.",
        "Keymap" => "Keymap editing is not available yet in the Rust interface.",
        "Keycaps" => "Keycap editing is not available yet in the Rust interface.",
        _ => "Parts library editing is not available yet in the Rust interface.",
    };
    rsx! { section { class: "m1-placeholder-workspace", h1 { "{name}" }, p { "{message}" }, button { onclick: move |_| workspace.set("Layout"), "Back to Layout" } } }
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

fn durability_state(durability: &Durability) -> &'static str {
    match durability {
        Durability::Saved { .. } => "saved",
        Durability::Saving { .. } => "saving",
        Durability::Failed { .. } => "failed",
        _ => "pending",
    }
}

#[component]
fn Library() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let recovery_required =
        runtime.model().lifecycle == boardstudio_application::Lifecycle::RecoveryRequired;
    let mut saved = use_signal(Vec::<(String, String)>::new);
    let accepted_identity = runtime
        .model()
        .accepted
        .as_ref()
        .map(|snapshot| (snapshot.document.id.clone(), snapshot.document.name.clone()));
    let list_runtime = runtime.clone();
    use_effect(use_reactive!(|accepted_identity| {
        let _ = accepted_identity;
        let runtime = list_runtime.clone();
        spawn_local(async move {
            match runtime.store.list_documents().await {
                Ok(documents) => saved.set(documents.into_iter().map(|d| (d.id, d.name)).collect()),
                Err(error) => runtime.report(error.to_string()),
            }
        });
    }));
    let reviung = runtime.clone();
    let sofle = runtime.clone();
    let import = runtime.clone();
    rsx! {
        section { class: "m1-library", "aria-label": "Keyboard library",
            button { onclick: move |_| { close_project_menu(); reviung.open_fixture("reviung41"); }, "REVIUNG41 copy" }
            button { onclick: move |_| { close_project_menu(); sofle.open_fixture("sofle"); }, "Sofle v2 copy" }
            label { "Import .boardstudio"
                input { r#type: "file", accept: ".boardstudio", onchange: move |event: FormEvent| {
                    let Some(input) = event.data().try_as_web_event().and_then(|e| e.target()).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) else { return; };
                    let Some(file) = input.files().and_then(|files| files.get(0)) else { return; };
                    close_project_menu();
                    import.import_file(file);
                    input.set_value("");
                }}
            }
            for (id, name) in saved() {
                button { key: "{id}", onclick: { let runtime = runtime.clone(); move |_| { close_project_menu(); runtime.open_saved(id.clone()); } }, if recovery_required { "Recover from {name} (discard pending changes)" } else { "{name}" } }
            }
        }
    }
}

#[component]
fn Editor() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let workspace = use_context::<WorkspaceState>().0;
    let layer_visibility = use_context::<LayerVisibility>();
    let active_workspace = workspace();
    let mut objects_open = use_signal(|| false);
    let mut inspect_open = use_signal(|| false);
    let model = runtime.model();
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
    let points: Vec<_> = visible.iter().map(|p| p.pose.at).collect();
    let min_x = points.iter().map(|p| p.x).reduce(f64::min).unwrap_or(-50.0) - 20.0;
    let max_x = points.iter().map(|p| p.x).reduce(f64::max).unwrap_or(50.0) + 20.0;
    let min_y = points.iter().map(|p| p.y).reduce(f64::min).unwrap_or(-50.0) - 20.0;
    let max_y = points.iter().map(|p| p.y).reduce(f64::max).unwrap_or(50.0) + 20.0;
    let width = (max_x - min_x).max(50.0) / model.camera.zoom;
    let height = (max_y - min_y).max(50.0) / model.camera.zoom;
    let view_x = (min_x + max_x - width) * 0.5 + model.camera.center.x;
    let view_y = -(min_y + max_y + height) * 0.5 - model.camera.center.y;
    let view_box = format!("{view_x} {view_y} {width} {height}");
    let svg = use_hook(|| Rc::new(RefCell::new(None::<SvgElement>)));
    let drag = use_hook(|| Rc::new(RefCell::new(None::<Drag>)));
    let space_down = use_hook(|| Rc::new(Cell::new(false)));
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
            let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height) else {
                return;
            };
            if !current.active {
                let dx = f64::from(pointer.client_x()) - current.client_x;
                let dy = f64::from(pointer.client_y()) - current.client_y;
                if dx * dx + dy * dy < 16.0 {
                    return;
                }
                let operation = runtime.operation();
                runtime.submit(Event::GestureBegin {
                    operation_id: operation,
                    pointer_id: current.pointer,
                    target_ids: current.positions.iter().map(|p| p.id.clone()).collect(),
                    transaction_id: format!("drag-{}", operation.0),
                    start: current.positions.clone(),
                    pitch: Vec2 { x: 19.05, y: 19.05 },
                    snap_fraction: 0.25,
                    geometry_snap: true,
                    gap: None,
                    alt: pointer.alt_key(),
                });
                if runtime.model().gesture.is_none() {
                    drag.borrow_mut().take();
                    return;
                }
                current.active = true;
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
            if current.pan {
                let surface = svg.borrow();
                let Some(element) = surface.as_ref() else {
                    return;
                };
                let rect = element.get_bounding_client_rect();
                if rect.width() > 0.0 && rect.height() > 0.0 {
                    let scale = (rect.width() / width).min(rect.height() / height);
                    if scale <= 0.0 {
                        return;
                    }
                    let camera = runtime.model().camera;
                    runtime.submit(Event::SetCamera {
                        operation_id: runtime.operation(),
                        center: Vec2 {
                            x: current.camera.x
                                - (f64::from(pointer.client_x()) - current.client_x) / scale,
                            y: current.camera.y
                                + (f64::from(pointer.client_y()) - current.client_y) / scale,
                        },
                        zoom: camera.zoom,
                    });
                }
                return;
            }
            if current.pan {
                drag.borrow_mut().take();
            } else if current.active
                && let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height)
            {
                runtime.submit(Event::GestureEnd {
                    pointer_id: current.pointer,
                    final_positions: moved(&current, point),
                    alt: pointer.alt_key(),
                });
            } else if current.active {
                runtime.submit(Event::GestureCancel {
                    pointer_id: current.pointer,
                });
            }
            drag.borrow_mut().take();
        }
    };
    let cancel_pointer = {
        let runtime = runtime.clone();
        let drag = drag.clone();
        move |_| {
            if let Some(current) = drag.borrow_mut().take()
                && current.active
                && !current.pan
            {
                runtime.submit(Event::GestureCancel {
                    pointer_id: current.pointer,
                });
            }
        }
    };
    let undo = runtime.clone();
    let redo = runtime.clone();
    let retry = runtime.clone();
    let recover = runtime.clone();
    use_effect(use_reactive((&active_workspace,), {
        let runtime = runtime.clone();
        let drag = drag.clone();
        move |_| {
            if let Some(current) = drag.borrow_mut().take()
                && current.active
                && !current.pan
            {
                runtime.submit(Event::GestureCancel {
                    pointer_id: current.pointer,
                });
            }
        }
    }));
    let keyboard = {
        let runtime = runtime.clone();
        let drag = drag.clone();
        let space_down = space_down.clone();
        move |event: KeyboardEvent| {
            let key = event.data().key().to_string();
            let code = event.data().code().to_string();
            let modifiers = event.data().modifiers();
            if key == " " || code == "Space" {
                space_down.set(true);
                event.prevent_default();
            } else if key == "Escape" {
                if let Some(current) = drag.borrow_mut().take()
                    && current.active
                    && !current.pan
                {
                    runtime.submit(Event::GestureCancel {
                        pointer_id: current.pointer,
                    });
                }
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
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            if !space_down.get() || pointer.button() != 0 {
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
        move |event: WheelEvent| {
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
    rsx! {
        section { class: "m1-editor", "aria-label": "Keyboard editor",
            nav { class: "m1-compact-panel-controls", "aria-label": "Panel visibility",
                button { "aria-controls": "m1-objects-panel", "aria-expanded": "{objects_open()}", onclick: move |_| objects_open.set(!objects_open()), "Objects" }
                if active_workspace == "Layout" {
                    button { "aria-controls": "m1-inspector-panel", "aria-expanded": "{inspect_open()}", onclick: move |_| inspect_open.set(!inspect_open()), "Inspect" }
                }
            }
            div { class: "m1-editor-body",
                div { id: "m1-objects-panel", class: if objects_open() { "m1-object-slot compact-open" } else { "m1-object-slot compact-closed" }, Objects {} }
                section { class: "m1-workspace-content", role: "tabpanel", id: "m1-workspace-panel", "aria-labelledby": "m1-tab-{active_workspace}",
                    if active_workspace == "Layout" {
                        div { class: "m1-canvas-toolbar",
                            span { "{document.name}" }
                            { let footprint_pressed = (layer_visibility.footprints)() && !(layer_visibility.hidden)().contains("Footprints"); rsx! {
                                button { class: "m1-footprints-toggle", aria_pressed: "{footprint_pressed}", onclick: move |_| { let mut footprints = layer_visibility.footprints; footprints.set(!footprints()); }, "Footprints" }
                            } }
                            if let Durability::Failed { reason, .. } = &model.durability {
                                p { role: "alert", class: "m1-save-error", "Save failed: {reason}" }
                                button { onclick: move |_| retry.submit(Event::RetrySave { operation_id: retry.operation() }), "Retry save" }
                            }
                            if model.lifecycle == boardstudio_application::Lifecycle::RecoveryRequired {
                                button { onclick: move |_| recover.recover_saved(), "Reopen last saved version (discard pending changes)" }
                            }
                        }
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
                                            let member_definition = member.and_then(|part| definitions.get(part.definition_id.as_str()).copied());
                                            let base_definition = definitions.get(matrix.definition_id.as_str()).copied();
                                            let size = member.and_then(|part| part.keycap).or_else(|| member_definition.and_then(|definition| definition.keycap)).or_else(|| base_definition.and_then(|definition| definition.keycap)).unwrap_or(Vec2 {
                                                x: (matrix.pitch.x - matrix.edge_gap.map(|gap| gap.x).unwrap_or(1.0)).max(1.0),
                                                y: (matrix.pitch.y - matrix.edge_gap.map(|gap| gap.y).unwrap_or(1.0)).max(1.0),
                                            });
                                            let pose = cell.pose;
                                            let selected = cell.member_id.as_ref().is_some_and(|id| model.selected_part_ids.contains(id));
                                            rsx! { rect { class: if selected { "m1-matrix-key is-selected" } else { "m1-matrix-key" }, x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}", rx: "0.9", transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation})", "data-matrix-id": "{matrix.id}", "data-row": "{cell.row}", "data-column": "{cell.column}" } }
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
                                                projected.cells.iter().find(|cell| cell.enabled && cell.member_id.as_deref() == Some(part.id.as_str())).map(|_| Vec2 {
                                                    x: (matrix.pitch.x - matrix.edge_gap.map(|gap| gap.x).unwrap_or(1.0)).max(1.0),
                                                    y: (matrix.pitch.y - matrix.edge_gap.map(|gap| gap.y).unwrap_or(1.0)).max(1.0),
                                                })
                                            })
                                        })
                                    } else { None }
                                };
                                let show_keycap = keycap.filter(|_| !(layer_visibility.hidden)().contains("Keycaps"));
                                let footprints_on = (layer_visibility.footprints)() && !(layer_visibility.hidden)().contains("Footprints");
                                let id = part.id.clone();
                                let runtime = runtime.clone(); let svg = svg.clone(); let drag = drag.clone(); let space_down = space_down.clone();
                                let range_ids = visible_ids.clone();
                                rsx! { if layer_visible { g { key: "{part.id}", class: "m1-scene-part", transform: "translate({pose.at.x},{pose.at.y}) rotate({pose.rotation}) {side_transform}", "data-part-id": "{part.id}",
                                    onpointerdown: move |event: PointerEvent| {
                                        let Some(pointer) = event.data().try_as_web_event() else { return; };
                                        if pointer.button() != 0 { return; }
                                        pointer.prevent_default(); pointer.stop_propagation();
                                        if let Some(svg) = svg.borrow().as_ref() { let _ = svg.set_pointer_capture(pointer.pointer_id()); let options = web_sys::FocusOptions::new(); options.set_prevent_scroll(true); let _ = svg.focus_with_options(&options); }
                                        if space_down.get() {
                                            *drag.borrow_mut() = Some(Drag { pointer: i64::from(pointer.pointer_id()), origin: Vec2::default(), client_x: f64::from(pointer.client_x()), client_y: f64::from(pointer.client_y()), positions: vec![], active: true, pan: true, camera: runtime.model().camera.center });
                                            return;
                                        }
                                        let Some(origin) = coordinates(&svg, &pointer, view_x, view_y, width, height) else { return; };
                                        let mode = if pointer.shift_key() { SelectionMode::Range } else if pointer.ctrl_key() || pointer.meta_key() { SelectionMode::Toggle } else { SelectionMode::Replace };
                                        let current = runtime.model();
                                        if !current.selected_part_ids.contains(&id) || mode != SelectionMode::Replace {
                                            let range_part_ids = if mode == SelectionMode::Range { range_ids.as_ref().clone() } else { vec![] };
                                            runtime.submit(Event::SelectParts { operation_id: runtime.operation(), part_ids: vec![id.clone()], range_part_ids, mode });
                                        }
                                        let current = runtime.model();
                                        if !current.selected_part_ids.contains(&id) { return; }
                                        let Some(snapshot) = current.accepted else { return; };
                                        let positions: Vec<_> = snapshot.document.parts.iter().filter(|p| current.selected_part_ids.contains(&p.id)).map(|p| Position { id: p.id.clone(), at: p.pose.at }).collect();
                                        *drag.borrow_mut() = Some(Drag { pointer: i64::from(pointer.pointer_id()), origin, client_x: f64::from(pointer.client_x()), client_y: f64::from(pointer.client_y()), positions, active: false, pan: false, camera: Vec2::default() });
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
                    } else if active_workspace == "Case" {
                        CasePanel {}
                    } else if active_workspace == "Export" {
                        ExportPanel {}
                    } else {
                        PlaceholderWorkspace { name: active_workspace }
                    }
                }
                if active_workspace == "Layout" {
                    div { id: "m1-inspector-panel", class: if inspect_open() { "m1-inspector-slot compact-open" } else { "m1-inspector-slot compact-closed" }, Inspector {} }
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

#[component]
fn Inspector() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let model = runtime.model();
    let selected = model
        .accepted
        .as_ref()
        .and_then(|s| {
            s.document
                .parts
                .iter()
                .find(|p| model.selected_part_ids.contains(&p.id))
        })
        .cloned();
    let mut x = use_signal(String::new);
    let mut y = use_signal(String::new);
    let numeric_edit = use_hook(|| Rc::new(RefCell::new(None::<NumericEdit>)));
    use_drop({
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        move || {
            let pending = numeric_edit.borrow_mut().take();
            if let Some(edit) = pending {
                let start = edit.start;
                submit_position(&runtime, &numeric_edit, edit, start, EditPhase::Preview);
            }
        }
    });
    let key = selected
        .as_ref()
        .map(|p| (p.id.clone(), p.pose.at.x, p.pose.at.y));
    use_effect(use_reactive((&key,), {
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        move |(key,)| {
            if let Some((_, px, py)) = key {
                x.set(px.to_string());
                y.set(py.to_string());
            }
            let changed_target = numeric_edit
                .borrow()
                .as_ref()
                .is_some_and(|edit| key.as_ref().is_none_or(|(id, _, _)| id != &edit.id));
            if changed_target && let Some(edit) = numeric_edit.borrow_mut().take() {
                let start = edit.start;
                submit_position(
                    &runtime,
                    &Rc::new(RefCell::new(None)),
                    edit,
                    start,
                    EditPhase::Preview,
                );
            }
        }
    }));
    let submit = {
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        move |_| {
            commit_numeric(&runtime, &numeric_edit, &x(), &y());
        }
    };
    let cancel_numeric: KeyboardHandler = {
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        let mut x = x;
        let mut y = y;
        Rc::new(RefCell::new(Box::new(move |event: KeyboardEvent| {
            let key = event.data().key().to_string();
            if key == "Escape" {
                event.prevent_default();
                let edit = numeric_edit.borrow_mut().take();
                if let Some(edit) = edit {
                    let start = edit.start;
                    submit_position(&runtime, &numeric_edit, edit, start, EditPhase::Preview);
                }
                if let Some(part) = runtime.model().accepted.and_then(|s| {
                    s.document
                        .parts
                        .iter()
                        .find(|p| runtime.model().selected_part_ids.contains(&p.id))
                        .cloned()
                }) {
                    let restored_x = part.pose.at.x.to_string();
                    let restored_y = part.pose.at.y.to_string();
                    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                        if let Some(input) = document
                            .get_element_by_id("m1-position-x")
                            .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
                        {
                            input.set_value(&restored_x);
                        }
                        if let Some(input) = document
                            .get_element_by_id("m1-position-y")
                            .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
                        {
                            input.set_value(&restored_y);
                        }
                    }
                    x.set(restored_x);
                    y.set(restored_y);
                }
                runtime.report("Position preview canceled.");
            } else if key == "Enter" {
                event.prevent_default();
                commit_numeric(&runtime, &numeric_edit, &x(), &y());
            }
        })))
    };
    rsx! { aside { class: "m1-inspector", "aria-label": "Inspect",
        header { h2 { "Inspect" } }
        h2 { "Position" }
        if let Some(part) = selected {
            p { "{part.reference}" }
            label { "X (mm)" input { id: "m1-position-x", r#type: "number", step: "any", value: "{x}", onkeydown: { let cancel = cancel_numeric.clone(); move |event| (cancel.borrow_mut())(event) }, oninput: { let runtime = runtime.clone(); let numeric_edit = numeric_edit.clone(); move |event: FormEvent| {
                x.set(event.value());
                let (Ok(px), Ok(py)) = (event.value().parse::<f64>(), y().parse::<f64>()) else { return; };
                if !px.is_finite() || !py.is_finite() { return; }
                let Some(snapshot) = runtime.model().accepted else { return; };
                let Some(part) = snapshot.document.parts.iter().find(|p| runtime.model().selected_part_ids.contains(&p.id)) else { return; };
                let edit = numeric_edit.borrow_mut().take().unwrap_or_else(|| NumericEdit { id: part.id.clone(), revision: snapshot.document.revision, transaction_id: format!("position-{}", runtime.operation().0), start: part.pose.at });
                submit_position(&runtime, &numeric_edit, edit, Vec2 { x: px, y: py }, EditPhase::Preview);
            }} } }
            label { "Y (mm)" input { id: "m1-position-y", r#type: "number", step: "any", value: "{y}", onkeydown: { let cancel = cancel_numeric.clone(); move |event| (cancel.borrow_mut())(event) }, oninput: { let runtime = runtime.clone(); let numeric_edit = numeric_edit.clone(); move |event: FormEvent| {
                y.set(event.value());
                let (Ok(px), Ok(py)) = (x().parse::<f64>(), event.value().parse::<f64>()) else { return; };
                if !px.is_finite() || !py.is_finite() { return; }
                let Some(snapshot) = runtime.model().accepted else { return; };
                let Some(part) = snapshot.document.parts.iter().find(|p| runtime.model().selected_part_ids.contains(&p.id)) else { return; };
                let edit = numeric_edit.borrow_mut().take().unwrap_or_else(|| NumericEdit { id: part.id.clone(), revision: snapshot.document.revision, transaction_id: format!("position-{}", runtime.operation().0), start: part.pose.at });
                submit_position(&runtime, &numeric_edit, edit, Vec2 { x: px, y: py }, EditPhase::Preview);
            }} } }
            button { onclick: submit, "Apply position" }
            if numeric_edit.borrow().is_some() { p { role: "status", "Preview only. Press Enter or Apply position to save, or Escape to cancel." } }
        } else { p { "Select a component to edit its position." } }
    }}
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

fn submit_position(
    runtime: &Rc<Runtime>,
    current: &Rc<RefCell<Option<NumericEdit>>>,
    edit: NumericEdit,
    at: Vec2,
    phase: EditPhase,
) {
    runtime.submit(Event::Edit {
        operation_id: runtime.operation(),
        command: EditCommand {
            base_revision: edit.revision,
            transaction_id: edit.transaction_id.clone(),
            phase,
            target_ids: vec![edit.id.clone()],
            operation: EditOperation::MoveParts {
                positions: vec![Position {
                    id: edit.id.clone(),
                    at,
                }],
            },
        },
    });
    if phase == EditPhase::Preview && at != edit.start {
        *current.borrow_mut() = Some(edit);
    } else {
        current.borrow_mut().take();
    }
}

fn commit_numeric(
    runtime: &Rc<Runtime>,
    current: &Rc<RefCell<Option<NumericEdit>>>,
    x: &str,
    y: &str,
) {
    let (Ok(x), Ok(y)) = (x.parse::<f64>(), y.parse::<f64>()) else {
        runtime.report("Enter finite X and Y coordinates.");
        return;
    };
    if !x.is_finite() || !y.is_finite() {
        runtime.report("Enter finite X and Y coordinates.");
        return;
    }
    let model = runtime.model();
    let Some(snapshot) = model.accepted else {
        return;
    };
    let Some(part) = snapshot
        .document
        .parts
        .iter()
        .find(|part| model.selected_part_ids.contains(&part.id))
    else {
        return;
    };
    let start = part.pose.at;
    let edit = current.borrow_mut().take().unwrap_or_else(|| NumericEdit {
        id: part.id.clone(),
        revision: snapshot.document.revision,
        transaction_id: format!("position-{}", runtime.operation().0),
        start,
    });
    submit_position(runtime, current, edit, Vec2 { x, y }, EditPhase::Commit);
}
