//! Presentation drafts and DOM input are separate from the durable session state.
mod canvas_interaction;
mod canvas_layers;
mod case_assembly_layers;
mod case_bodies;
mod case_controller;
mod case_display;
mod case_viewer;
mod case_workspace;
mod context_summary;
mod firmware_positions;
mod inspector;
mod instance_selection;
mod keycaps_finding_marker;
mod keycaps_fit;
mod keycaps_navigation;
mod keycaps_scene;
mod keycaps_settings;
mod keycaps_workspace;
mod keymap;
mod keymap_workspace;
mod layout_camera;
#[cfg(test)]
mod layout_component_inspector_tests;
mod layout_viewer;
pub(crate) mod layout_viewer_source;
mod layout_workspace;
mod library;
mod mechanical_settings;
mod mechanical_settings_controller;
mod mechanical_settings_mount;
pub(crate) mod model_delivery;
mod objects;
mod outline_lifecycle;
mod outline_snapping;
mod panels;
mod part_placement;
mod parts;
mod parts_import_footprint;
mod parts_workspace;
mod pcb_layers;
mod pcb_module_footprints;
mod pcb_physical_setup;
mod pcb_scene;
mod pcb_wiring;
mod pcb_workspace;
mod selection;
mod setup_guide;
mod shared_viewer;
mod workspace_composition;
mod zmk_firmware_export;

use crate::case_generation_lifecycle::AutomaticCaseGeneration;
use canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner};
use canvas_layers::CanvasLayers;
pub(crate) use case_viewer::{CasePreviewViewer, CaseViewer};
use library::Library;
pub(crate) use mechanical_settings::{MechanicalSettings, MechanicalSettingsProps};
pub(crate) use mechanical_settings_mount::MechanicalSettingsMount;
use panels::{
    InspectorPanel, ObjectsPanel, PanelMode, PanelSettings, PanelSide, use_panel_settings,
};
use parts::{PartsQuery, PartsSelection};
use selection::{ReentrancyReset, SelectionAdapter};
use setup_guide::{PendingNewKeyboard, SetupGuidePreferences, SetupGuideRequest, SetupGuideStage};
use zmk_firmware_export::{
    ZmkFirmwareExportPanelInput, ZmkFirmwareExportRow, use_export_panel_input,
};
mod footprint_graphics;

use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, ReadModel, Scope, SelectionMode, SnapshotToken,
    TerminalOutcome,
};
use boardstudio_core::model::{
    Constraint, Contour, EditCommand, EditOperation, EditPhase, Matrix, MatrixSplayAffect, Part,
    PartDefinition, PartKind, PartOutline, Position, Vec2,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct LayoutOwnerIdentity {
    scope: Option<Scope>,
    token: Option<SnapshotToken>,
    revision: Option<u64>,
    generation: u64,
    workspace: &'static str,
}

fn case_setup_context_is_current(
    identity: &pcb_physical_setup::OwnerIdentity,
    strict: bool,
    workspace: &str,
    scope: Option<&Scope>,
) -> bool {
    identity.context == pcb_physical_setup::OwnerContext::CaseInspector
        && workspace == "Case"
        && scope.is_some_and(|scope| {
            scope.session_epoch == identity.session_epoch
                && scope.document_id == identity.document_id
                && scope.board_id == identity.board_id
                && (!strict || scope.instance_id == identity.instance_id)
        })
}

#[cfg(test)]
mod physical_setup_owner_tests {
    use super::*;
    use boardstudio_application::SessionEpoch;
    use pcb_physical_setup::{OwnerContext, OwnerIdentity};
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn case_transport_owner_does_not_require_a_part_tree_selection() {
        let scope = Scope {
            session_epoch: SessionEpoch(7),
            document_id: "sofle".into(),
            board_id: "right-pcb".into(),
            instance_id: Some("right-half".into()),
        };
        let identity = OwnerIdentity {
            context: OwnerContext::CaseInspector,
            session_epoch: scope.session_epoch,
            document_id: scope.document_id.clone(),
            board_id: scope.board_id.clone(),
            instance_id: scope.instance_id.clone(),
            token: SnapshotToken(23),
            revision: 21,
            generation: 4,
        };
        assert!(
            case_setup_context_is_current(&identity, true, "Case", Some(&scope)),
            "physical setup belongs to the active Case assembly scope even when no part is selected"
        );
    }
}

#[derive(Clone)]
struct OwnedTreeCellAnchor {
    owner: LayoutOwnerIdentity,
    cell: objects::TreeCellAnchor,
}

#[derive(Clone, Copy)]
pub(super) struct WorkspaceState(pub(super) Signal<&'static str>);
/// The explicit UI preference is separate from Session's effective instance.
#[derive(Clone, Copy)]
pub(crate) struct InstanceSelection(Signal<Option<instance_selection::Preference>>);

#[derive(Clone, Copy)]
pub(crate) struct CaseGenerationState {
    pub(crate) live_preview: Signal<bool>,
    pub(crate) automatic: Signal<AutomaticCaseGeneration>,
}

impl InstanceSelection {
    pub(crate) fn is_current(self, model: &boardstudio_application::ReadModel) -> bool {
        instance_selection::is_current(model, self.0.read().as_ref())
    }

    pub(crate) fn reconcile(
        mut self,
        session_epoch: boardstudio_application::SessionEpoch,
        document_id: String,
        explicit_id: String,
    ) {
        self.0.set(Some(instance_selection::Preference {
            session_epoch,
            document_id,
            explicit_id,
        }));
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
pub(crate) fn use_empty_test_instance_selection() {
    let preference = use_signal(|| None);
    use_context_provider(|| InstanceSelection(preference));
}

#[cfg(all(test, target_arch = "wasm32"))]
pub(crate) fn use_test_case_generation_state() {
    let live_preview = use_signal(|| true);
    let automatic = use_signal(AutomaticCaseGeneration::new);
    use_context_provider(|| CaseGenerationState {
        live_preview,
        automatic,
    });
}

#[derive(Clone, Copy)]
struct ThemeState(Signal<&'static str>);
#[derive(Clone, Copy)]
struct ResolvedTheme(Memo<&'static str>);
#[derive(Clone, Copy)]
struct LayerVisibility {
    hidden: Signal<BTreeSet<String>>,
    modules_hidden: Signal<BTreeSet<String>>,
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
    keycaps_finding: EventHandler<keycaps_fit::FindingNavigationRequest>,
    pcb_empty_hit: EventHandler<PointerEvent>,
    pcb_part_hit: EventHandler<pcb_scene::PcbPartHit>,
    pcb_part_pointer_down: EventHandler<pcb_scene::PcbPartPointerDown>,
    pcb_wiring_edit_board: EventHandler<()>,
    layout_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pcb_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pcb_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pcb_transform_properties: EventHandler<()>,
    pcb_part_position: EventHandler<pcb_wiring::PcbPartPositionAction>,
    case_action: EventHandler<case_workspace::TreeAction>,
    case_display: EventHandler<case_workspace::DisplayRequest>,
    keymap_layer: EventHandler<String>,
    keymap_export: EventHandler<()>,
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
    let project_created = use_signal(|| None::<SetupGuideRequest>);
    let pending_new_keyboard = use_signal(|| None::<PendingNewKeyboard>);
    let new_keyboard_error = use_signal(String::new);
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
    let observed_version = version();
    use_context_provider(|| runtime.clone());
    use_context_provider(|| version);
    use_context_provider(|| project_created);
    use_context_provider(|| pending_new_keyboard);
    use_context_provider(|| new_keyboard_error);
    use_context_provider(|| adapter.clone());
    let mut workspace = use_signal(|| "Layout");
    use_context_provider(|| WorkspaceState(workspace));
    let case_generation = CaseGenerationState {
        live_preview: use_signal(|| true),
        automatic: use_signal(AutomaticCaseGeneration::new),
    };
    use_context_provider(|| case_generation);
    let runtime_for_creation = runtime.clone();
    use_effect(use_reactive!(|observed_version| {
        let _ = observed_version;
        let mut pending_new_keyboard = pending_new_keyboard;
        let mut project_created = project_created;
        let mut new_keyboard_error = new_keyboard_error;
        let Some(pending) = pending_new_keyboard() else {
            return;
        };
        let Some(outcome) = pending.outcome.borrow().clone() else {
            return;
        };
        let model = runtime_for_creation.model();
        match outcome {
            TerminalOutcome::Completed => {
                let settlement = crate::setup_guide_state::creation_settlement(
                    &pending.project_id,
                    model
                        .accepted
                        .as_ref()
                        .map(|snapshot| snapshot.document.id.as_str()),
                    model.lifecycle == Lifecycle::Ready
                        && model.accepted.as_ref().is_some_and(|snapshot| {
                            model.durability
                                == Durability::Saved {
                                    revision: snapshot.document.revision,
                                }
                        }),
                );
                match settlement {
                    crate::setup_guide_state::CreationSettlement::Wait => return,
                    crate::setup_guide_state::CreationSettlement::Retire => {
                        new_keyboard_error.set(
                            "The new keyboard was superseded before it could open. Try again."
                                .into(),
                        );
                        pending_new_keyboard.set(None);
                        return;
                    }
                    crate::setup_guide_state::CreationSettlement::Reveal => {}
                }
                project_created.set(Some(SetupGuideRequest {
                    project_id: pending.project_id.clone(),
                    request_id: pending.project_id,
                    start_at_project: true,
                }));
                pending_new_keyboard.set(None);
                new_keyboard_error.set(String::new());
            }
            TerminalOutcome::Rejected(message)
            | TerminalOutcome::PersistenceFailed(message)
            | TerminalOutcome::BlockedByRecovery(message)
            | TerminalOutcome::ExecutorFailed(message) => {
                new_keyboard_error.set(message);
                pending_new_keyboard.set(None);
            }
            TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
                new_keyboard_error.set("The new keyboard could not be opened. Try again.".into());
                pending_new_keyboard.set(None);
            }
        }
    }));
    let layer_visibility = LayerVisibility {
        hidden: use_signal(BTreeSet::new),
        modules_hidden: use_signal(pcb_module_footprints::default_hidden_layers),
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
        link { rel: "stylesheet", href: "assets/firmware-keymap-panel.css" }
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
                    Library { project_menu: true }
                }
                span { class: "m1-save-state", "data-state": "{save_state}", "{save_label}" }
                WorkspaceNavigation {}
                button { class: "m1-export-tab", id: "m1-tab-Export", "aria-pressed": "{workspace() == \"Export\"}", onclick: move |_| workspace.set("Export"), "Export" }
                ThemePicker {}
            }
            if runtime.model().accepted.is_some() { Editor {} }
            else { LibraryLanding {} }
                RuntimeReportBanner {}
            if !new_keyboard_error().is_empty() {
                p { role: "alert", class: "m1-status", "{new_keyboard_error()}" }
            }
        }
    }
}

#[component]
fn RuntimeReportBanner() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let alert = runtime.status_is_alert();
    let status = runtime.status();
    rsx! {
        if alert || status != "Saved locally." {
            p {
                role: if alert { "alert" } else { "status" },
                "aria-live": if alert { "assertive" } else { "polite" },
                class: "m1-status",
                "{status}"
            }
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
            Library { project_menu: false }
        }
    }
}

#[component]
fn ExportPanel(#[props(default)] zmk_firmware: Option<ZmkFirmwareExportPanelInput>) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let model = runtime.model();
    let archive = runtime.clone();
    let preference = runtime.clone();
    let step = runtime.clone();
    rsx! {
        section { class: "m1-export-panel", "aria-label": "Export",
            h1 { "Export" }
            p { "Create files from the saved keyboard in the current board and instance scope." }
            div { class: "m1-export-actions",
                button { disabled: model.accepted.is_none(), onclick: move |_| step.export_step(), "Export STEP" }
            }
            if let Some(firmware) = zmk_firmware {
                h2 { "Design files" }
                div { class: "m1-export-list",
                    ZmkFirmwareExportRow { ready: firmware.ready, on_export: firmware.on_export }
                }
            }
            section { class: "m1-export-portable", "aria-label": "Portable project",
                h2 { "Portable project" }
                p { "Keep an editable copy of the whole project, including all boards." }
                label { class: "m1-export-option",
                    input {
                        r#type: "checkbox",
                        aria_label: "Embed used models",
                        checked: runtime.embed_used_models(),
                        onchange: move |event: FormEvent| preference.set_embed_used_models(event.checked()),
                    }
                    span { strong { "Embed used models" } small { "Include attached 3D model files used in this project." } }
                }
                button {
                    class: "m1-export-action",
                    disabled: model.accepted.is_none(),
                    onclick: move |_| archive.export_project_copy(),
                    "Save .boardstudio project"
                }
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

fn current_case_layer_ids(
    runtime: &Runtime,
    scope: &Scope,
    snapshot: &AcceptedSnapshot,
) -> Option<Rc<[String]>> {
    let scene = runtime.cad_scene()?;
    if !scene.exact
        || scene.scope != *scope
        || scene.token != snapshot.token
        || scene.prepared.revision != snapshot.document.revision
    {
        return None;
    }
    Some(Rc::from(
        scene
            .mechanical
            .as_ref()
            .map(|assembly| {
                assembly
                    .stack
                    .iter()
                    .map(|layer| layer.id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    ))
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

fn keymap_toolbar_height() -> f64 {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| {
            document
                .query_selector(".m1-design-canvas-toolbar")
                .ok()
                .flatten()
        })
        .map(|toolbar| toolbar.get_bounding_client_rect().height())
        .filter(|height| height.is_finite() && *height > 0.0)
        .unwrap_or(42.0)
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

fn pcb_bounds(snapshot: &AcceptedSnapshot, scope: &Scope) -> Option<(f64, f64, f64, f64)> {
    if snapshot.session_epoch != scope.session_epoch || snapshot.document.id != scope.document_id {
        return None;
    }
    let board = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)?;
    let member_ids: BTreeSet<&str> = board.part_ids.iter().map(String::as_str).collect();
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    let mut include = |x: f64, y: f64| {
        bounds = Some(bounds.map_or((x, x, y, y), |(min_x, max_x, min_y, max_y)| {
            (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
        }));
    };
    for contour in accepted_board_contours(snapshot, &scope.board_id).iter() {
        for point in &contour.points {
            include(point.x, point.y);
        }
    }
    for part in snapshot
        .document
        .parts
        .iter()
        .filter(|part| member_ids.contains(part.id.as_str()))
    {
        let pose = snapshot
            .scene
            .transforms
            .iter()
            .find(|transform| transform.id == part.id)
            .map(|transform| transform.pose)
            .unwrap_or(part.pose);
        include(pose.at.x, pose.at.y);
    }
    bounds
}

fn active_board_scope_matches(model: &boardstudio_application::ReadModel, scope: &Scope) -> bool {
    model.active_board_id == scope.board_id
        && model.active_instance_id == scope.instance_id
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.session_epoch == scope.session_epoch
                && snapshot.document.id == scope.document_id
        })
}

fn current_layout_owner(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
) -> LayoutOwnerIdentity {
    let model = runtime.model();
    LayoutOwnerIdentity {
        scope: runtime.scope(),
        token: model.accepted.as_ref().map(|snapshot| snapshot.token),
        revision: model
            .accepted
            .as_ref()
            .map(|snapshot| snapshot.document.revision),
        generation: (adapter.generation)(),
        workspace: workspace(),
    }
}

fn layout_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    if current_layout_owner(runtime, workspace, adapter) != *owner || owner.workspace != "Layout" {
        return false;
    }
    let model = runtime.model();
    owner
        .scope
        .as_ref()
        .is_some_and(|scope| active_board_scope_matches(&model, scope))
}

fn pcb_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    if current_layout_owner(runtime, workspace, adapter) != *owner || owner.workspace != "PCB" {
        return false;
    }
    let model = runtime.model();
    owner
        .scope
        .as_ref()
        .is_some_and(|scope| active_board_scope_matches(&model, scope))
}

fn canvas_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    match owner.workspace {
        "Layout" => layout_owner_is_current(runtime, workspace, adapter, owner),
        "PCB" => pcb_owner_is_current(runtime, workspace, adapter, owner),
        _ => false,
    }
}

fn layout_component_inspector_projection(
    model: &ReadModel,
    selected: Option<&objects::ScopedTreeContext>,
    context_generation: u64,
    scope_generation: u64,
) -> Option<inspector::LayoutComponentInspectorProjection> {
    let selected = selected?;
    let objects::TreeContext::Component {
        part_id: Some(part_id),
        matrix_id: None,
        assembly_id: None,
        ..
    } = &selected.context
    else {
        return None;
    };
    if model.selected_part_ids.len() != 1
        || model.selected_part_ids.first() != Some(part_id)
        || !selection::context_is_current(model, &selected.scope, &selected.context)
        || objects::component_context_for_finding_part(model, part_id).as_ref()
            != Some(&selected.context)
    {
        return None;
    }
    let snapshot = model.accepted.as_ref()?;
    let document = &snapshot.document;
    let part = document.parts.iter().find(|part| part.id == *part_id)?;
    let definition = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)?;
    let board = document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)?;
    if !board.part_ids.iter().any(|id| id == part_id) {
        return None;
    }
    let board_parts: Vec<_> = board
        .part_ids
        .iter()
        .filter_map(|id| {
            document
                .parts
                .iter()
                .find(|part| part.id == *id)
                .map(|part| inspector::BoardPartChoice {
                    id: part.id.clone(),
                    reference: part.reference.clone(),
                })
        })
        .collect();
    let layouts: Vec<_> = document
        .layouts
        .iter()
        .filter(|layout| layout.board_id == board.id)
        .map(|layout| inspector::LayoutChoice {
            id: layout.id.clone(),
            name: layout.name.clone(),
        })
        .collect();
    let active_layout = document
        .layouts
        .iter()
        .find(|layout| layout.board_id == board.id && layout.part_ids.contains(part_id));
    let paired_layout = active_layout.and_then(|layout| {
        layout
            .mirror_link
            .as_ref()
            .and_then(|link| {
                document
                    .layouts
                    .iter()
                    .find(|item| item.id == link.source_id)
            })
            .or_else(|| {
                document.layouts.iter().find(|item| {
                    item.mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == layout.id)
                })
            })
    });
    let active_constraint = document
        .constraints
        .iter()
        .find(|constraint| {
            constraint.target() == part.id.as_str()
                && board.part_ids.iter().any(|id| id == constraint.source())
        })
        .cloned();
    let relationship_summary = if let Some(partner) = paired_layout {
        format!(
            "Key assemblies, diodes and components mirror with {}. Replace a component on one half to keep it local.",
            partner.name
        )
    } else if active_layout.is_some() {
        "This layout is independent. Its geometry and components can be edited separately."
            .to_owned()
    } else if let Some(constraint) = active_constraint.as_ref() {
        let source = board_parts
            .iter()
            .find(|candidate| candidate.id == constraint.source())
            .map(|candidate| candidate.reference.as_str())
            .unwrap_or("A part");
        format!("{source} drives {}.", part.reference)
    } else {
        "No saved placement relationship on this selection.".to_owned()
    };
    let definition_kind = match &definition.kind {
        PartKind::Switch => "switch",
        PartKind::Controller => "controller",
        PartKind::Connector => "connector",
        PartKind::Encoder => "encoder",
        PartKind::Passive => "passive",
        PartKind::Custom => "custom",
        PartKind::Utility => "utility",
    };
    Some(inspector::LayoutComponentInspectorProjection {
        owner: inspector::LayoutComponentInspectorOwner {
            scope: selected.scope.clone(),
            snapshot_token: snapshot.token,
            revision: document.revision,
            context_generation,
            scope_generation,
            part_id: part.id.clone(),
        },
        reference: part.reference.clone(),
        definition_name: definition.name.clone(),
        definition_kind: definition_kind.to_owned(),
        envelope_notice: definition.envelope_notice.clone(),
        locked: part.locked.unwrap_or(false),
        position: part.pose.at,
        layout_id: active_layout.map(|layout| layout.id.clone()),
        layouts,
        outline: part.outline.clone().unwrap_or_else(PartOutline::default),
        board_parts,
        active_constraint,
        relationship_summary,
    })
}

fn update_layout_selection_kind_for_tree_context(
    mut selection_kind: Signal<objects::LayoutSelectionKind>,
    context: &objects::TreeContext,
) {
    if matches!(
        context,
        objects::TreeContext::Component {
            part_id: Some(_),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        }
    ) {
        selection_kind.set(objects::LayoutSelectionKind::Part);
    }
}

fn layout_component_inspector_owner_key(
    model: &ReadModel,
    workspace: &'static str,
    selected: Option<&objects::ScopedTreeContext>,
) -> Option<inspector::LayoutComponentInspectorOwnerKey> {
    if workspace != "Layout" {
        return None;
    }
    let selected = selected?;
    let objects::TreeContext::Component {
        part_id: Some(part_id),
        matrix_id: None,
        assembly_id: None,
        ..
    } = &selected.context
    else {
        return None;
    };
    model.accepted.as_ref()?;
    Some(inspector::LayoutComponentInspectorOwnerKey {
        scope: Some(selected.scope.clone()),
        workspace,
        part_id: Some(part_id.clone()),
    })
}

fn layout_component_inspector_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    layout_owner: &LayoutOwnerIdentity,
    lifetime: &inspector::LayoutComponentInspectorLifetime,
    owner: &inspector::LayoutComponentInspectorOwner,
) -> bool {
    if !layout_owner_is_current(runtime, workspace, adapter, layout_owner)
        || owner.scope_generation != (adapter.generation)()
        || owner.context_generation != lifetime.current_generation()
        || layout_owner.scope.as_ref() != Some(&owner.scope)
        || layout_owner.token != Some(owner.snapshot_token)
        || layout_owner.revision != Some(owner.revision)
    {
        return false;
    }
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return false;
    };
    if snapshot.token != owner.snapshot_token || snapshot.document.revision != owner.revision {
        return false;
    }
    let selected = adapter.selected_context.read();
    let Some(selected) = selected.as_ref() else {
        return false;
    };
    selected.scope == owner.scope
        && matches!(
            &selected.context,
            objects::TreeContext::Component {
                part_id: Some(part_id),
                matrix_id: None,
                assembly_id: None,
                ..
            } if part_id == &owner.part_id
        )
        && selection::context_is_current(&model, &owner.scope, &selected.context)
        && model.selected_part_ids.len() == 1
        && model.selected_part_ids.first() == Some(&owner.part_id)
}

fn submit_layout_component_edit(
    runtime: &Rc<Runtime>,
    owner: &inspector::LayoutComponentInspectorOwner,
    target_ids: Vec<String>,
    operation: EditOperation,
) {
    let operation_id = runtime.operation();
    runtime.submit(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: owner.revision,
            transaction_id: format!("layout-component-inspector-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids,
            operation,
        },
    });
}

fn dispatch_layout_component_inspector_action(
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    layout_owner: &LayoutOwnerIdentity,
    lifetime: &inspector::LayoutComponentInspectorLifetime,
    mut workspace: Signal<&'static str>,
    mut inspect_open: Signal<bool>,
    action: inspector::LayoutComponentInspectorAction,
) {
    let action_owner = match &action {
        inspector::LayoutComponentInspectorAction::SetPosition { owner, .. }
        | inspector::LayoutComponentInspectorAction::AssignLayout { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetOutline { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetConstraint { owner, .. }
        | inspector::LayoutComponentInspectorAction::RemoveConstraint { owner, .. }
        | inspector::LayoutComponentInspectorAction::NavigateElectrical { owner } => owner,
    };
    if !layout_component_inspector_owner_is_current(
        runtime,
        workspace,
        adapter,
        layout_owner,
        lifetime,
        action_owner,
    ) {
        return;
    }
    let current = runtime.model();
    let Some(snapshot) = current.accepted.as_ref() else {
        return;
    };
    let Some(board) = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == action_owner.scope.board_id)
    else {
        return;
    };
    if !board
        .part_ids
        .iter()
        .any(|part_id| part_id == &action_owner.part_id)
    {
        return;
    }
    match action {
        inspector::LayoutComponentInspectorAction::SetPosition { owner, axis, value } => {
            if !value.is_finite() {
                return;
            }
            let Some(part) = snapshot
                .document
                .parts
                .iter()
                .find(|part| part.id == owner.part_id)
            else {
                return;
            };
            let mut at = part.pose.at;
            match axis {
                inspector::ComponentPositionAxis::X => at.x = value,
                inspector::ComponentPositionAxis::Y => at.y = value,
            }
            submit_layout_component_edit(
                runtime,
                &owner,
                vec![owner.part_id.clone()],
                EditOperation::MoveParts {
                    positions: vec![Position {
                        id: owner.part_id.clone(),
                        at,
                    }],
                },
            );
        }
        inspector::LayoutComponentInspectorAction::AssignLayout { owner, layout_id } => {
            if let Some(layout_id) = layout_id.as_ref()
                && !snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|layout| layout.id == *layout_id && layout.board_id == board.id)
            {
                return;
            }
            let current_layout = snapshot.document.layouts.iter().find(|layout| {
                layout.board_id == board.id && layout.part_ids.contains(&owner.part_id)
            });
            if current_layout.map(|layout| layout.id.as_str()) == layout_id.as_deref() {
                return;
            }
            let mut replacement = snapshot.document.as_ref().clone();
            for layout in &mut replacement.layouts {
                if layout.board_id == board.id {
                    layout.part_ids.retain(|part_id| part_id != &owner.part_id);
                }
            }
            if let Some(layout_id) = layout_id.as_ref()
                && let Some(layout) = replacement
                    .layouts
                    .iter_mut()
                    .find(|layout| layout.id == *layout_id && layout.board_id == board.id)
            {
                layout.part_ids.push(owner.part_id.clone());
            }
            let mut target_ids = vec![owner.part_id.clone()];
            if let Some(layout_id) = layout_id {
                target_ids.push(layout_id);
            }
            submit_layout_component_edit(
                runtime,
                &owner,
                target_ids,
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
            );
        }
        inspector::LayoutComponentInspectorAction::SetOutline { owner, outline } => {
            let mut replacement = snapshot.document.as_ref().clone();
            let Some(part) = replacement
                .parts
                .iter_mut()
                .find(|part| part.id == owner.part_id)
            else {
                return;
            };
            part.outline = Some(outline);
            submit_layout_component_edit(
                runtime,
                &owner,
                vec![owner.part_id.clone()],
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
            );
        }
        inspector::LayoutComponentInspectorAction::SetConstraint {
            owner,
            source_part_id,
            values,
        } => {
            if source_part_id == owner.part_id
                || !board.part_ids.iter().any(|id| id == &source_part_id)
            {
                return;
            }
            let existing_id = snapshot
                .document
                .constraints
                .iter()
                .find(|constraint| {
                    constraint.target() == owner.part_id
                        && board.part_ids.iter().any(|id| id == constraint.source())
                })
                .map(|constraint| constraint.id().to_owned());
            let constraint_id = existing_id.unwrap_or_else(|| {
                format!("layout-component-constraint-{}", runtime.operation().0)
            });
            let constraint = match values {
                inspector::LayoutConstraintValues::Offset { offset, rotation } => {
                    Constraint::Offset {
                        id: constraint_id,
                        source_part_id,
                        target_part_id: owner.part_id.clone(),
                        offset,
                        rotation,
                    }
                }
                inspector::LayoutConstraintValues::Mirror { axis, coordinate } => {
                    Constraint::Mirror {
                        id: constraint_id,
                        source_part_id,
                        target_part_id: owner.part_id.clone(),
                        axis,
                        coordinate,
                    }
                }
            };
            let mut target_ids = vec![constraint.id().to_owned(), owner.part_id.clone()];
            if !target_ids.iter().any(|id| id == constraint.source()) {
                target_ids.push(constraint.source().to_owned());
            }
            submit_layout_component_edit(
                runtime,
                &owner,
                target_ids,
                EditOperation::SetConstraint { constraint },
            );
        }
        inspector::LayoutComponentInspectorAction::RemoveConstraint {
            owner,
            constraint_id,
        } => {
            if !snapshot.document.constraints.iter().any(|constraint| {
                constraint.id() == constraint_id && constraint.target() == owner.part_id
            }) {
                return;
            }
            submit_layout_component_edit(
                runtime,
                &owner,
                vec![constraint_id.clone(), owner.part_id.clone()],
                EditOperation::RemoveConstraint { id: constraint_id },
            );
        }
        inspector::LayoutComponentInspectorAction::NavigateElectrical { .. } => {
            workspace.set("PCB");
            inspect_open.set(true);
        }
    }
}

fn layout_view_mode_handler(
    is_owner_current: impl Fn() -> bool + 'static,
    is_assembly_3d: Signal<bool>,
    mut set_assembly_3d: impl FnMut(bool) + 'static,
    placement: part_placement::PartPlacementMount,
    canvas_interaction: CanvasInteractionArbiter,
    before_placement_cancel: impl Fn() + 'static,
    after_placement_cancel: impl Fn() + 'static,
) -> EventHandler<bool> {
    EventHandler::new(move |assembly_3d| {
        if !is_owner_current() {
            return;
        }
        if assembly_3d && !is_assembly_3d() {
            before_placement_cancel();
            if placement.busy || placement.projection.is_some() {
                placement.on_cancel.call(());
            }
            after_placement_cancel();
            match canvas_interaction.current() {
                Some(CanvasInteractionOwner::PartPlacement) => {
                    canvas_interaction.release(CanvasInteractionOwner::PartPlacement);
                }
                Some(CanvasInteractionOwner::OutlinePerimeter) => {
                    canvas_interaction.release(CanvasInteractionOwner::OutlinePerimeter);
                }
                Some(CanvasInteractionOwner::MirroredPair) | None => {}
            }
        }
        set_assembly_3d(assembly_3d);
    })
}

/// Keymap and Keycaps only change views; their shared 3D viewer has no Layout
/// placement or transform interaction to cancel. Keep these requests bound to
/// the exact workspace, scope, accepted snapshot, and selection generation.
fn design_consumer_view_mode_handler(
    runtime: Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    is_assembly_3d: Signal<bool>,
) -> EventHandler<bool> {
    EventHandler::new(move |assembly_3d| {
        if owner.workspace == "Layout"
            || current_layout_owner(&runtime, workspace, &adapter) != owner
            || owner.workspace != workspace()
        {
            return;
        }
        let model = runtime.model();
        if !owner.scope.as_ref().is_some_and(|scope| {
            active_board_scope_matches(&model, scope) && runtime.scope().as_ref() == Some(scope)
        }) {
            return;
        }
        let mut is_assembly_3d = is_assembly_3d;
        is_assembly_3d.set(assembly_3d);
    })
}

fn tree_cell_anchor_for_owner(
    anchor: &Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    owner: &LayoutOwnerIdentity,
) -> Option<objects::TreeCellAnchor> {
    anchor
        .borrow()
        .as_ref()
        .filter(|stored| stored.owner == *owner)
        .map(|stored| stored.cell.clone())
}

fn update_tree_cell_anchor(
    anchor: &Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    owner: &LayoutOwnerIdentity,
    model: &boardstudio_application::ReadModel,
    context: Option<&objects::TreeContext>,
) {
    if owner.workspace != "Layout" && owner.workspace != "PCB" {
        anchor.borrow_mut().take();
        return;
    }
    let (Some(scope), Some(token), Some(revision), Some(context)) =
        (owner.scope.as_ref(), owner.token, owner.revision, context)
    else {
        anchor.borrow_mut().take();
        return;
    };
    if !active_board_scope_matches(model, scope)
        || model.accepted.as_ref().is_none_or(|snapshot| {
            snapshot.token != token || snapshot.document.revision != revision
        })
    {
        anchor.borrow_mut().take();
        return;
    }
    let exact_cell = match context {
        objects::TreeContext::Key {
            matrix_id,
            row,
            column,
        }
        | objects::TreeContext::Component {
            matrix_id: Some(matrix_id),
            row: Some(row),
            column: Some(column),
            ..
        } => Some(objects::TreeCellAnchor {
            matrix_id: matrix_id.clone(),
            row: *row,
            column: *column,
        }),
        _ => None,
    };
    if let Some(cell) = exact_cell {
        if objects::context_for_cell(model, &cell.matrix_id, cell.row, cell.column).is_some() {
            *anchor.borrow_mut() = Some(OwnedTreeCellAnchor {
                owner: owner.clone(),
                cell,
            });
            return;
        }
        anchor.borrow_mut().take();
        return;
    }
    let retained = tree_cell_anchor_for_owner(anchor, owner);
    let compatible = retained.as_ref().is_some_and(|cell| {
        objects::context_for_selection_kind(
            model,
            context,
            objects::LayoutSelectionKind::Key,
            Some(cell),
        )
        .is_some()
    });
    if !compatible {
        anchor.borrow_mut().take();
    }
}

fn matrix_snap_parameters(
    model: &boardstudio_application::ReadModel,
    scope: &Scope,
    adapter: &SelectionAdapter,
    retained_cell: Option<&objects::TreeCellAnchor>,
) -> (Option<Vec2>, Option<f64>) {
    let Some(snapshot) = model.accepted.as_ref() else {
        return (None, None);
    };
    let context = adapter
        .selected_context
        .read()
        .clone()
        .filter(|selected| {
            selected.scope == *scope
                && selection::context_is_current(model, scope, &selected.context)
        })
        .map(|selected| selected.context)
        .or_else(|| {
            model
                .selected_part_ids
                .first()
                .and_then(|part_id| objects::context_for_part(model, part_id))
        });
    let Some(context) = context else {
        return (None, None);
    };
    let matrix_id = match &context {
        objects::TreeContext::Matrix { matrix_id }
        | objects::TreeContext::Row { matrix_id, .. }
        | objects::TreeContext::Column { matrix_id, .. }
        | objects::TreeContext::Key { matrix_id, .. }
        | objects::TreeContext::Component {
            matrix_id: Some(matrix_id),
            ..
        } => matrix_id,
        objects::TreeContext::Component {
            part_id: Some(_),
            matrix_id: None,
            ..
        } => {
            let Some(cell) = retained_cell.filter(|cell| {
                objects::context_for_selection_kind(
                    model,
                    &context,
                    objects::LayoutSelectionKind::Key,
                    Some(cell),
                )
                .is_some()
            }) else {
                return (None, None);
            };
            &cell.matrix_id
        }
        objects::TreeContext::Outline { .. }
        | objects::TreeContext::OutlineVersion { .. }
        | objects::TreeContext::Bridge { .. }
        | objects::TreeContext::Board { .. }
        | objects::TreeContext::LayoutGroup { .. }
        | objects::TreeContext::Component { .. } => return (None, None),
    };
    let Some(matrix) = snapshot
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == *matrix_id)
    else {
        return (None, None);
    };
    (Some(matrix.pitch), matrix.edge_gap.map(|gap| gap.x))
}

fn durability_state(durability: &Durability) -> &'static str {
    match durability {
        Durability::Saved { .. } => "saved",
        Durability::Saving { .. } => "saving",
        Durability::Failed { .. } => "failed",
        _ => "pending",
    }
}

fn pin_inspector_on_desktop(mut settings: Signal<PanelSettings>) {
    if is_compact_viewport() {
        return;
    }
    let mut current = settings();
    if current.mode != PanelMode::Pinned {
        current.mode = PanelMode::Pinned;
        settings.set(current);
    }
}

fn is_compact_viewport() -> bool {
    web_sys::window()
        .and_then(|window| window.match_media("(max-width: 760px)").ok().flatten())
        .is_some_and(|query| query.matches())
}

fn browse_parts_workspace(
    mut workspace: Signal<&'static str>,
    mut compact_open: Signal<bool>,
    mut settings: Signal<PanelSettings>,
    compact: bool,
) {
    workspace.set("Parts");
    if compact {
        compact_open.set(true);
        return;
    }
    let mut current = settings();
    if current.mode != PanelMode::Pinned {
        current.mode = PanelMode::Pinned;
        settings.set(current);
    }
}

fn focus_first_inspector_control_on_next_frame(
    alive: Rc<Cell<bool>>,
    owner: keycaps_navigation::NavigationOwner,
    current_owner: impl Fn() -> keycaps_navigation::LiveNavigationOwner + 'static,
) {
    let Some(window) = web_sys::window() else {
        return;
    };
    keycaps_navigation::queue_owner_focus(
        alive,
        owner,
        current_owner,
        || {
            if let Some(element) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| {
                    if let Some(body) = document
                        .query_selector("#m1-inspector-panel-content .m1-inspector-body")
                        .ok()
                        .flatten()
                        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
                    {
                        body.set_scroll_top(0);
                    }
                    document
                        .query_selector(
                            "#m1-inspector-panel-content :is(button, input):not(:disabled)",
                        )
                        .ok()
                        .flatten()
                })
                .and_then(|element| element.dyn_into::<HtmlElement>().ok())
            {
                let _ = element.focus();
            }
        },
        move |task| {
            let callback = Closure::once_into_js(task);
            let _ = window.request_animation_frame(callback.unchecked_ref());
        },
    );
}

fn current_keycaps_navigation_owner(
    workspace: Signal<&'static str>,
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    body_selection: Signal<Option<case_viewer::BodySelection>>,
    case_selection: case_viewer::CaseSelection,
) -> keycaps_navigation::LiveNavigationOwner {
    let scope = runtime.scope();
    let model = runtime.model();
    let (token, revision) = model
        .accepted
        .as_ref()
        .map(|snapshot| (Some(snapshot.token), Some(snapshot.document.revision)))
        .unwrap_or((None, None));
    let destinations = match (workspace(), scope.as_ref()) {
        ("Layout", Some(scope)) => adapter
            .selected_context
            .read()
            .as_ref()
            .filter(|selected| &selected.scope == scope)
            .map(|selected| {
                vec![keycaps_navigation::Destination::Layout(
                    selected.context.clone(),
                )]
            })
            .unwrap_or_default(),
        ("Case", Some(scope)) => {
            let layer_id = case_selection.layer_id(scope);
            let body_id = body_selection
                .read()
                .as_ref()
                .filter(|selected| &selected.scope == scope)
                .map(|body| body.body_id.clone());
            keycaps_navigation::active_case_destination(&layer_id, body_id.as_deref())
                .into_iter()
                .collect()
        }
        _ => Vec::new(),
    };
    keycaps_navigation::LiveNavigationOwner {
        workspace: workspace(),
        scope,
        generation: (adapter.generation)(),
        token,
        revision,
        destinations,
    }
}

fn fit_selected_bridge(runtime: &Rc<Runtime>, bridge_id: Option<&str>) {
    let Some(bridge_id) = bridge_id else { return };
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return;
    };
    let Some(bridge) = snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == model.active_board_id)
        .and_then(|scene| scene.bridges.iter().find(|bridge| bridge.id == bridge_id))
    else {
        return;
    };
    let Some((min_x, max_x, min_y, max_y)) = bridge.points.iter().fold(None, |bounds, point| {
        Some(bounds.map_or(
            (point.x, point.x, point.y, point.y),
            |(min_x, max_x, min_y, max_y): (f64, f64, f64, f64)| {
                (
                    min_x.min(point.x),
                    max_x.max(point.x),
                    min_y.min(point.y),
                    max_y.max(point.y),
                )
            },
        ))
    }) else {
        return;
    };
    let board_bounds = snapshot
        .scene
        .board_contours
        .iter()
        .filter(|board| board.board_id == model.active_board_id)
        .flat_map(|board| &board.contours)
        .flat_map(|contour| &contour.points)
        .fold(None, |bounds, point| {
            Some(bounds.map_or(
                (point.x, point.x, point.y, point.y),
                |(min_x, max_x, min_y, max_y): (f64, f64, f64, f64)| {
                    (
                        min_x.min(point.x),
                        max_x.max(point.x),
                        min_y.min(point.y),
                        max_y.max(point.y),
                    )
                },
            ))
        })
        .or_else(|| {
            let board = snapshot
                .document
                .boards
                .iter()
                .find(|board| board.id == model.active_board_id)?;
            let mut parts = snapshot
                .document
                .parts
                .iter()
                .filter(|part| board.part_ids.contains(&part.id));
            let first = parts.next()?.pose.at;
            Some(parts.fold(
                (first.x, first.x, first.y, first.y),
                |(min_x, max_x, min_y, max_y), part| {
                    (
                        min_x.min(part.pose.at.x),
                        max_x.max(part.pose.at.x),
                        min_y.min(part.pose.at.y),
                        max_y.max(part.pose.at.y),
                    )
                },
            ))
        })
        .unwrap_or((min_x, max_x, min_y, max_y));
    let viewport_bounds = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)
        .and_then(|board| {
            snapshot
                .document
                .parts
                .iter()
                .filter(|part| board.part_ids.contains(&part.id))
                .map(|part| part.pose.at)
                .fold(None, |bounds, point| {
                    Some(bounds.map_or(
                        (point.x, point.x, point.y, point.y),
                        |(min_x, max_x, min_y, max_y): (f64, f64, f64, f64)| {
                            (
                                min_x.min(point.x),
                                max_x.max(point.x),
                                min_y.min(point.y),
                                max_y.max(point.y),
                            )
                        },
                    ))
                })
        })
        .unwrap_or(board_bounds);
    let bridge_width = (max_x - min_x).max(bridge.width).max(1.0);
    let bridge_height = (max_y - min_y).max(bridge.width).max(1.0);
    let board_width = (board_bounds.1 - board_bounds.0).max(50.0) + 40.0;
    let board_height = (board_bounds.3 - board_bounds.2).max(50.0) + 40.0;
    let zoom =
        (0.72 * (board_width / bridge_width).min(board_height / bridge_height)).clamp(0.15, 8.0);
    runtime.submit(Event::SetCamera {
        operation_id: runtime.operation(),
        center: layout_camera::bridge_camera_offset(
            viewport_bounds,
            Vec2 {
                x: (min_x + max_x) * 0.5,
                y: (min_y + max_y) * 0.5,
            },
        ),
        zoom,
    });
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
        keycaps_finding: EventHandler::new(|_: keycaps_fit::FindingNavigationRequest| {}),
        pcb_empty_hit: EventHandler::new(|_: PointerEvent| {}),
        pcb_part_hit: EventHandler::new(|_: pcb_scene::PcbPartHit| {}),
        pcb_part_pointer_down: EventHandler::new(|_: pcb_scene::PcbPartPointerDown| {}),
        pcb_wiring_edit_board: EventHandler::new(|_: ()| {}),
        layout_selection_kind: EventHandler::new(|_: objects::LayoutSelectionKind| {}),
        layout_snap_intent: EventHandler::new(|_: objects::LayoutSnapIntent| {}),
        pcb_selection_kind: EventHandler::new(|_: objects::LayoutSelectionKind| {}),
        pcb_snap_intent: EventHandler::new(|_: objects::LayoutSnapIntent| {}),
        pcb_transform_properties: EventHandler::new(|_: ()| {}),
        pcb_part_position: EventHandler::new(|_: pcb_wiring::PcbPartPositionAction| {}),
        case_action: EventHandler::new(|_: case_workspace::TreeAction| {}),
        case_display: EventHandler::new(|_: case_workspace::DisplayRequest| {}),
        keymap_layer: EventHandler::new(|_: String| {}),
        keymap_export: EventHandler::new(|_: ()| {}),
        show_configured_board: EventHandler::new(|_: String| {}),
    });
    let runtime = use_context::<Rc<Runtime>>();
    let mut objects_open = use_signal(|| false);
    let mut inspect_open = use_signal(|| false);
    let objects_panel_settings = use_panel_settings(PanelSide::Objects);
    let inspector_panel_settings = use_panel_settings(PanelSide::Inspector);
    let created_request_signal = use_context::<Signal<Option<SetupGuideRequest>>>();
    let created_request = created_request_signal();
    let mut guide_preferences = use_signal(|| None::<SetupGuidePreferences>);
    let mut guide_name_draft = use_signal(|| None::<(String, String)>);
    let consumed_guide_requests = use_hook(|| Rc::new(RefCell::new(BTreeSet::<String>::new())));
    let accepted_project_id = runtime
        .model()
        .accepted
        .as_ref()
        .map(|snapshot| snapshot.document.id.clone());
    let mut guide_workspace = use_context::<WorkspaceState>().0;
    use_effect(use_reactive!(|accepted_project_id, created_request| {
        let mut created_request_signal = created_request_signal;
        let Some(project_id) = accepted_project_id.as_ref() else {
            return;
        };
        if let Some(request) = created_request
            .as_ref()
            .filter(|request| request.project_id == *project_id)
        {
            let already_consumed = consumed_guide_requests
                .borrow()
                .contains(&request.request_id);
            if !already_consumed {
                consumed_guide_requests
                    .borrow_mut()
                    .insert(request.request_id.clone());
                guide_preferences.set(Some(SetupGuidePreferences {
                    project_id: project_id.clone(),
                    open: true,
                    current_stage: if request.start_at_project {
                        SetupGuideStage::Project
                    } else {
                        guide_preferences()
                            .filter(|preferences| preferences.project_id == *project_id)
                            .map(|preferences| preferences.current_stage)
                            .unwrap_or_else(|| {
                                setup_guide::read_preferences(project_id).current_stage
                            })
                    },
                }));
                if request.start_at_project {
                    guide_workspace.set("Layout");
                }
                setup_guide::reveal_panels(
                    crate::setup_guide_state::GuideReveal::Guide,
                    objects_open,
                    inspect_open,
                    objects_panel_settings,
                    inspector_panel_settings,
                );
                created_request_signal.set(None);
            }
        } else if guide_preferences()
            .as_ref()
            .is_none_or(|preferences| preferences.project_id != *project_id)
        {
            let preferences = setup_guide::read_preferences(project_id);
            if preferences.open {
                setup_guide::reveal_panels(
                    crate::setup_guide_state::GuideReveal::Guide,
                    objects_open,
                    inspect_open,
                    objects_panel_settings,
                    inspector_panel_settings,
                );
            }
            guide_preferences.set(Some(preferences));
        }
    }));
    let preferences_to_persist = guide_preferences();
    use_effect(use_reactive!(|preferences_to_persist| {
        if let Some(preferences) = preferences_to_persist.as_ref() {
            setup_guide::write_preferences(preferences);
        }
    }));
    let last_accepted_name = use_hook(|| Rc::new(RefCell::new(None::<(String, String)>)));
    let project_name_for_draft = runtime
        .model()
        .accepted
        .as_ref()
        .map(|snapshot| (snapshot.document.id.clone(), snapshot.document.name.clone()));
    use_effect(use_reactive!(|project_name_for_draft| {
        if let Some(current) = project_name_for_draft {
            let changed = crate::setup_guide_state::accepted_name_change(
                last_accepted_name.borrow().as_ref(),
                &current,
            );
            *last_accepted_name.borrow_mut() = Some(current);
            if let Some(value) = changed {
                guide_name_draft.set(Some(value));
            }
        }
    }));
    let adapter = use_context::<SelectionAdapter>();
    let layout_component_inspector_lifetime =
        use_hook(|| Rc::new(inspector::LayoutComponentInspectorLifetime::default()));
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
    let focused_keycaps_finding = use_signal(|| None::<keycaps_finding_marker::FocusedFinding>);
    let pending_keycaps_navigation_fit =
        use_signal(|| None::<keycaps_navigation::PendingLayoutFit>);
    let keycaps_navigation_alive = keycaps_navigation::use_navigation_lifetime();
    let case_display = use_signal(std::collections::BTreeMap::new);
    let case_selection = case_viewer::CaseSelection {
        body: case_body_selection,
        layer: case_layer_selection,
        display: case_display,
    };
    use_context_provider(|| case_selection);
    let case_tree_expanded = use_signal(BTreeSet::<String>::new);
    let workspace = use_context::<WorkspaceState>().0;
    let active_workspace = workspace();
    let requested_workspace_panel =
        panels::use_workspace_panel_defaults(active_workspace, objects_open, inspect_open);
    let render_generation = (adapter.generation)();
    let layout_selection_kind = use_signal(objects::LayoutSelectionKind::default);
    let layout_snap_settings = use_signal(objects::LayoutSnapSettings::default);
    let mut layout_assembly_3d = use_signal(|| false);
    let tree_cell_anchor = use_hook(|| Rc::new(RefCell::new(None::<OwnedTreeCellAnchor>)));
    let matrix_inspector = objects::use_matrix_inspector(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let key_size = objects::use_key_size(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let matrix_splay_affect = use_signal(|| MatrixSplayAffect::Following);
    let matrix_transform_inspector = objects::use_workspace_matrix_transform(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
        matrix_splay_affect,
        "Layout",
    );
    let pcb_matrix_transform_inspector = objects::use_workspace_matrix_transform(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
        matrix_splay_affect,
        "PCB",
    );
    let layout_align = objects::use_canvas_align(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
        "Layout",
    );
    let pcb_align = objects::use_canvas_align(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
        "PCB",
    );
    let mut matrix_setup = objects::use_matrix_setup(
        runtime.clone(),
        version,
        adapter.selected_context,
        adapter.anchor_scope,
        workspace,
        adapter.generation,
    );
    let canvas_interaction = use_hook(CanvasInteractionArbiter::default);
    let pair_created_selection = use_signal(|| None::<objects::MirroredPairCreated>);
    let on_mirrored_pair_created = use_callback({
        let runtime = runtime.clone();
        let mut adapter = adapter.clone();
        let mut pair_created_selection = pair_created_selection;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        move |created: objects::MirroredPairCreated| {
            let current_scope = runtime.scope();
            if workspace() != "Layout"
                || (adapter.generation)() != created.owner.scope_generation
                || current_scope.as_ref() != Some(&created.owner.scope)
                || created.scope != created.owner.scope
            {
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if snapshot.token != created.result_token
                || snapshot.document.revision != created.result_revision
                || snapshot.token == created.owner.snapshot_token
                || snapshot.document.revision <= created.owner.revision
                || snapshot.document.id != created.owner.scope.document_id
                || snapshot.session_epoch != created.owner.scope.session_epoch
                || model.active_board_id != created.owner.board_id
                || model.active_instance_id != created.owner.scope.instance_id
                || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: created.result_revision,
                    })
                || !snapshot
                    .document
                    .matrices
                    .iter()
                    .any(|matrix| matrix.id == created.left_matrix_id)
                || !snapshot
                    .document
                    .matrices
                    .iter()
                    .any(|matrix| matrix.id == created.right_matrix_id)
                || !snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|layout| layout.id == created.left_layout_id)
                || !snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|layout| layout.id == created.right_layout_id)
            {
                return;
            }
            let context = objects::TreeContext::Matrix {
                matrix_id: created.left_matrix_id.clone(),
            };
            let Some(part_ids) = objects::resolve_selection(&model, &context)
                .filter(|part_ids| !part_ids.is_empty())
            else {
                return;
            };
            adapter
                .selected_context
                .set(Some(objects::ScopedTreeContext {
                    scope: created.scope.clone(),
                    context,
                }));
            adapter.anchor_scope.set(None);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids,
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
            pair_created_selection.set(Some(created));
            pin_inspector_on_desktop(inspector_panel_settings);
            objects_open.set(false);
            inspect_open.set(true);
        }
    });
    let mut mirrored_pair = objects::use_mirrored_pair(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        on_mirrored_pair_created,
        canvas_interaction.clone(),
    );
    if mirrored_pair.owns_canvas {
        matrix_setup.can_open = false;
    }
    if matrix_setup.projection.is_some() {
        mirrored_pair.can_open = false;
    }
    let (outline_inspector, outline_activation) = outline_lifecycle::use_outline_lifecycle(
        runtime.clone(),
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let layer_visibility = use_context::<LayerVisibility>();
    let parts_query: PartsQuery = use_signal(String::new);
    let parts_selection: PartsSelection = use_signal(|| None);
    let parts_selection_generation = use_signal(|| 0u64);
    use_context_provider(|| parts::PartsSelectionGeneration(parts_selection_generation));
    let parts_generator_draft = use_signal(|| None::<parts::GeneratorPreviewDraft>);
    use_context_provider(|| parts::GeneratorDraftStore(parts_generator_draft));
    let layout_target: Signal<Option<String>> = use_signal(|| None);
    let mut keymap_layer_id = use_signal(|| "base".to_owned());
    let has_inspector = matches!(
        active_workspace,
        "Layout" | "Parts" | "Keymap" | "Keycaps" | "Case" | "PCB"
    );
    let on_parts_select = {
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        move |_| {
            objects_open.set(false);
            inspect_open.set(true);
        }
    };
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
    let layout_owner = LayoutOwnerIdentity {
        scope: current_scope.clone(),
        token: accepted_token,
        revision: model
            .accepted
            .as_ref()
            .map(|snapshot| snapshot.document.revision),
        generation: render_generation,
        workspace: active_workspace,
    };
    use_effect(use_reactive((&layout_owner,), {
        let tree_cell_anchor = tree_cell_anchor.clone();
        move |(owner,)| {
            let stale = tree_cell_anchor
                .borrow()
                .as_ref()
                .is_some_and(|anchor| anchor.owner != owner);
            if stale {
                tree_cell_anchor.borrow_mut().take();
            }
        }
    }));
    let active_board_id = model.active_board_id.clone();
    keycaps_finding_marker::use_retire_stale_finding(
        focused_keycaps_finding,
        layout_owner.workspace,
        layout_owner.scope.clone(),
        layout_owner.token,
        layout_owner.revision,
        active_board_id.clone(),
    );
    let layer_source =
        current_scope
            .clone()
            .zip(model.accepted.as_ref())
            .map(|(scope, snapshot)| keymap::LayerSource {
                scope,
                token: snapshot.token,
                revision: snapshot.document.revision,
            });
    let keycaps_edit_source = current_scope
        .as_ref()
        .filter(|scope| {
            active_board_scope_matches(&model, scope) && instance_selection.is_current(&model)
        })
        .and_then(|scope| {
            model
                .accepted
                .as_ref()
                .map(|snapshot| keycaps_settings::KeycapsEditSource {
                    scope: scope.clone(),
                    token: snapshot.token,
                    revision: snapshot.document.revision,
                })
        });
    let keycaps_fit_source = current_scope
        .as_ref()
        .filter(|scope| active_board_scope_matches(&model, scope))
        .and_then(|scope| {
            model.accepted.as_ref().map(|snapshot| {
                let case_preview_current = runtime.cad_scene().is_some_and(|scene| {
                    scene.exact
                        && scene.scope == *scope
                        && scene.token == snapshot.token
                        && scene.prepared.revision == snapshot.document.revision
                });
                keycaps_fit::KeycapsFitSource {
                    scope: scope.clone(),
                    token: snapshot.token,
                    revision: snapshot.document.revision,
                    case_preview_current,
                }
            })
        });
    let keycaps_fit_state = keycaps_fit::use_keycaps_fit(runtime.clone(), keycaps_fit_source);
    let keycaps_settings_actions = keycaps_settings::use_keycaps_settings_actions(
        runtime.clone(),
        keycaps_edit_source,
        workspace,
        adapter.generation,
    );
    let pcb_wiring_mount = pcb_wiring::use_pcb_wiring_controller(runtime.clone(), version);
    let pcb_part_net_actions = pcb_wiring::use_pcb_part_net_edits(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
    );
    let firmware_position_actions = pcb_wiring::use_firmware_position_edits(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_mount.resolution_signal,
    );
    let part_input_actions = pcb_wiring::use_part_input_edits(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
    );
    // Read guide state at every admission: an async operation may outlive the rendered guide
    // stage or switch to a different accepted project.
    let project_setup_active: Rc<dyn Fn() -> bool> = Rc::new({
        let runtime = runtime.clone();
        move || {
            let model = runtime.model();
            let Some(project_id) = model
                .accepted
                .as_ref()
                .map(|snapshot| snapshot.document.id.as_str())
            else {
                return false;
            };
            guide_preferences
                .peek()
                .as_ref()
                .is_some_and(|preferences| {
                    preferences.open
                        && preferences.current_stage == SetupGuideStage::Project
                        && preferences.project_id == project_id
                })
        }
    });
    let physical_setup_mount = pcb_physical_setup::use_controller(
        runtime.clone(),
        version,
        adapter.generation,
        project_setup_active.clone(),
        instance_selection,
        Rc::new({
            let runtime = runtime.clone();
            let generation = adapter.generation;
            let project_setup_active = project_setup_active;
            move |identity: &pcb_physical_setup::OwnerIdentity, strict: bool| {
                let model = runtime.model();
                let Some(accepted) = model.accepted.as_ref() else {
                    return false;
                };
                let context_current = match identity.context {
                    pcb_physical_setup::OwnerContext::ProjectGuide => {
                        project_setup_active()
                            && model.active_board_id == identity.board_id
                            && (!strict || model.active_instance_id == identity.instance_id)
                    }
                    pcb_physical_setup::OwnerContext::CaseInspector => {
                        case_setup_context_is_current(
                            identity,
                            strict,
                            workspace(),
                            runtime.scope().as_ref(),
                        )
                    }
                };
                context_current
                    && generation() == identity.generation
                    && accepted.session_epoch == identity.session_epoch
                    && accepted.document.id == identity.document_id
                    && (!strict
                        || (accepted.token == identity.token
                            && accepted.document.revision == identity.revision
                            && (identity.context
                                != pcb_physical_setup::OwnerContext::CaseInspector
                                || instance_selection.is_current(&model))))
            }
        }),
        Rc::new(|accepted, intent| {
            Box::pin(parts::prepare_physical_setup_proposal(accepted, intent))
        }),
    );
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
        let owner = layout_owner.clone();
        let tree_cell_anchor = tree_cell_anchor.clone();
        let generation = render_generation;
        let mut workspace = workspace;
        let selection_kind = layout_selection_kind;
        let inspector_settings = inspector_panel_settings;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        move |request: objects::TreeSelectRequest| {
            if request
                .outline_action
                .as_ref()
                .is_some_and(|action| !action.is_current(&runtime, generation))
                || (adapter.generation)() != generation
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
                objects::TreeContext::Outline { .. }
                | objects::TreeContext::OutlineVersion { .. }
                | objects::TreeContext::Bridge { .. } => (true, true),
                objects::TreeContext::LayoutGroup { .. } => (true, false),
                objects::TreeContext::Matrix { .. }
                | objects::TreeContext::Row { .. }
                | objects::TreeContext::Column { .. }
                | objects::TreeContext::Key { .. }
                | objects::TreeContext::Component { .. } => (true, true),
            };
            let outline_route = matches!(
                &request.context,
                objects::TreeContext::Outline { .. }
                    | objects::TreeContext::OutlineVersion { .. }
                    | objects::TreeContext::Bridge { .. }
            );
            let bridge_id = match &request.context {
                objects::TreeContext::Bridge { bridge_id, .. } => Some(bridge_id.clone()),
                _ => None,
            };
            let activation = request.outline_action.clone();
            let scope = request.scope.clone();
            update_layout_selection_kind_for_tree_context(selection_kind, &request.context);
            update_tree_cell_anchor(
                &tree_cell_anchor,
                &owner,
                &runtime.model(),
                Some(&request.context),
            );
            selection::submit_context(&runtime, &adapter, request);
            if (adapter.generation)() != generation || runtime.scope().as_ref() != Some(&scope) {
                return;
            }
            if let Some(action) = activation {
                workspace.set("Layout");
                outline_activation.call(action);
            }
            if outline_route {
                workspace.set("Layout");
                fit_selected_bridge(&runtime, bridge_id.as_deref());
                pin_inspector_on_desktop(inspector_settings);
            }
            if close_objects {
                objects_open.set(false);
            }
            if open_inspect {
                inspect_open.set(true);
            }
        }
    };
    let on_layout_selection_kind = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let tree_cell_anchor = tree_cell_anchor.clone();
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        let mut selection_kind = layout_selection_kind;
        move |kind: objects::LayoutSelectionKind| {
            if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
                return;
            }
            selection_kind.set(kind);
            let model = runtime.model();
            let Some(scope) = owner.scope.as_ref() else {
                tree_cell_anchor.borrow_mut().take();
                return;
            };
            let Some(selected) = adapter
                .selected_context
                .read()
                .clone()
                .filter(|selected| selected.scope == *scope)
            else {
                tree_cell_anchor.borrow_mut().take();
                return;
            };
            if !selection::context_is_current(&model, scope, &selected.context) {
                tree_cell_anchor.borrow_mut().take();
                return;
            }
            let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
            let Some(projection) = objects::context_for_selection_kind(
                &model,
                &selected.context,
                kind,
                retained.as_ref(),
            ) else {
                return;
            };
            if !selection::context_is_current(&model, scope, &projection.context) {
                return;
            }
            update_tree_cell_anchor(&tree_cell_anchor, &owner, &model, Some(&projection.context));
            selection::submit_context(
                &runtime,
                &adapter,
                objects::TreeSelectRequest {
                    scope: scope.clone(),
                    context: projection.context,
                    mode: SelectionMode::Replace,
                    outline_action: None,
                },
            );
            if layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
                pin_inspector_on_desktop(inspector_panel_settings);
                objects_open.set(false);
                inspect_open.set(true);
            }
        }
    };
    let on_layout_snap_intent = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let mut snap_settings = layout_snap_settings;
        move |intent: objects::LayoutSnapIntent| {
            if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
                return;
            }
            let mut settings = snap_settings.read().clone();
            match intent {
                objects::LayoutSnapIntent::Fraction(value)
                    if [0.0, 0.125, 0.25, 0.5, 1.0, -1.0, -0.5, -0.1].contains(&value) =>
                {
                    settings.snap_fraction = value;
                }
                objects::LayoutSnapIntent::GeometrySnap(enabled) => {
                    settings.geometry_snap = enabled;
                }
                objects::LayoutSnapIntent::GapSnap(enabled) => {
                    settings.gap_snap = enabled;
                }
                objects::LayoutSnapIntent::GapOverride(value) => {
                    settings.gap_override = value;
                }
                objects::LayoutSnapIntent::Fraction(_) => return,
            }
            snap_settings.set(settings);
        }
    };
    let on_pcb_selection_kind = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let tree_cell_anchor = tree_cell_anchor.clone();
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        let mut selection_kind = layout_selection_kind;
        move |kind: objects::LayoutSelectionKind| {
            if !pcb_owner_is_current(&runtime, workspace, &adapter, &owner) {
                return;
            }
            selection_kind.set(kind);
            let model = runtime.model();
            let Some(scope) = owner.scope.as_ref() else {
                tree_cell_anchor.borrow_mut().take();
                return;
            };
            let Some(selected) = adapter
                .selected_context
                .read()
                .clone()
                .filter(|selected| selected.scope == *scope)
            else {
                tree_cell_anchor.borrow_mut().take();
                return;
            };
            if !selection::context_is_current(&model, scope, &selected.context) {
                tree_cell_anchor.borrow_mut().take();
                return;
            }
            let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
            let Some(projection) = objects::context_for_selection_kind(
                &model,
                &selected.context,
                kind,
                retained.as_ref(),
            ) else {
                return;
            };
            if !selection::context_is_current(&model, scope, &projection.context) {
                return;
            }
            update_tree_cell_anchor(&tree_cell_anchor, &owner, &model, Some(&projection.context));
            selection::submit_context(
                &runtime,
                &adapter,
                objects::TreeSelectRequest {
                    scope: scope.clone(),
                    context: projection.context,
                    mode: SelectionMode::Replace,
                    outline_action: None,
                },
            );
            if pcb_owner_is_current(&runtime, workspace, &adapter, &owner) {
                pin_inspector_on_desktop(inspector_panel_settings);
                objects_open.set(false);
                inspect_open.set(true);
            }
        }
    };
    let on_pcb_snap_intent = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let mut snap_settings = layout_snap_settings;
        move |intent: objects::LayoutSnapIntent| {
            if !pcb_owner_is_current(&runtime, workspace, &adapter, &owner) {
                return;
            }
            let mut settings = snap_settings.read().clone();
            match intent {
                objects::LayoutSnapIntent::Fraction(value)
                    if [0.0, 0.125, 0.25, 0.5, 1.0, -1.0, -0.5, -0.1].contains(&value) =>
                {
                    settings.snap_fraction = value;
                }
                objects::LayoutSnapIntent::GeometrySnap(enabled) => {
                    settings.geometry_snap = enabled;
                }
                objects::LayoutSnapIntent::GapSnap(enabled) => {
                    settings.gap_snap = enabled;
                }
                objects::LayoutSnapIntent::GapOverride(value) => {
                    settings.gap_override = value;
                }
                objects::LayoutSnapIntent::Fraction(_) => return,
            }
            snap_settings.set(settings);
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
            if !active_board_scope_matches(&model, &scope) {
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
            if !active_board_scope_matches(&current, &scope)
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
            if !active_board_scope_matches(&model, &scope) || !instance_selection.is_current(&model)
            {
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
            if !active_board_scope_matches(&current, &scope)
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
    let on_pcb_part_hit = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let scope = render_scope.clone();
        let generation = render_generation;
        let token = snapshot.token;
        let owner = layout_owner.clone();
        let tree_cell_anchor = tree_cell_anchor.clone();
        move |request: pcb_scene::PcbPartHit| {
            if workspace() != "PCB"
                || request.scope != scope
                || request.token != token
                || request.generation != generation
                || runtime.scope().as_ref() != Some(&scope)
                || (adapter.generation)() != generation
            {
                return;
            }
            let model = runtime.model();
            if !active_board_scope_matches(&model, &scope) || !instance_selection.is_current(&model)
            {
                return;
            }
            let Some(snapshot) = model
                .accepted
                .as_ref()
                .filter(|snapshot| snapshot.token == request.token)
            else {
                return;
            };
            let Some(board) = snapshot
                .document
                .boards
                .iter()
                .find(|board| board.id == scope.board_id)
            else {
                return;
            };
            if !board.part_ids.contains(&request.part_id)
                || !snapshot
                    .document
                    .parts
                    .iter()
                    .any(|part| part.id == request.part_id)
            {
                return;
            }
            let Some(hit_context) = objects::context_for_part(&model, &request.part_id) else {
                return;
            };
            if !selection::context_is_current(&model, &scope, &hit_context) {
                return;
            }
            update_tree_cell_anchor(&tree_cell_anchor, &owner, &model, Some(&hit_context));
            let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
            let context = objects::context_for_selection_kind(
                &model,
                &hit_context,
                layout_selection_kind(),
                retained.as_ref(),
            )
            .map(|projection| projection.context)
            .unwrap_or(hit_context);
            let mode = if request.range {
                SelectionMode::Range
            } else if request.additive {
                SelectionMode::Toggle
            } else {
                SelectionMode::Replace
            };
            let range_ids = if mode == SelectionMode::Range {
                selection::eligible_live_ids(&model)
                    .into_iter()
                    .filter(|id| board.part_ids.contains(id))
                    .collect()
            } else {
                Vec::new()
            };
            selection::submit_canvas_selection(
                &runtime, &adapter, &scope, generation, context, mode, range_ids,
            );
        }
    };
    let on_pcb_wiring_edit_board = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let scope = render_scope.clone();
        let generation = render_generation;
        let token = snapshot.token;
        move |()| {
            if workspace() != "PCB"
                || runtime.scope().as_ref() != Some(&scope)
                || (adapter.generation)() != generation
            {
                return;
            }
            let model = runtime.model();
            if !active_board_scope_matches(&model, &scope) || !instance_selection.is_current(&model)
            {
                return;
            }
            let Some(snapshot) = model
                .accepted
                .as_ref()
                .filter(|snapshot| snapshot.token == token)
            else {
                return;
            };
            if !snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == scope.board_id)
            {
                return;
            }
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
        }
    };
    let on_pcb_part_position = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let scope = render_scope.clone();
        let generation = render_generation;
        move |action: pcb_wiring::PcbPartPositionAction| {
            if !pcb_owner_is_current(&runtime, workspace, &adapter, &owner)
                || action.owner.scope != scope
                || Some(action.owner.token) != owner.token
                || Some(action.owner.revision) != owner.revision
                || action.owner.generation != generation
            {
                return;
            }
            let model = runtime.model();
            if model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: action.owner.revision,
                    })
                || model.display_preview.is_some()
                || model.gesture.is_some()
                || !active_board_scope_matches(&model, &action.owner.scope)
                || model.selected_part_ids.len() != 1
                || model.selected_part_ids.first() != Some(&action.owner.part_id)
                || adapter
                    .selected_context
                    .read()
                    .as_ref()
                    .is_none_or(|selected| {
                        selected.scope != action.owner.scope
                            || !matches!(
                                &selected.context,
                                objects::TreeContext::Component {
                                    part_id: Some(part_id),
                                    matrix_id: None,
                                    assembly_id: None,
                                    ..
                                } if part_id == &action.owner.part_id
                            )
                            || !selection::context_is_current(
                                &model,
                                &action.owner.scope,
                                &selected.context,
                            )
                    })
            {
                return;
            }
            let Some(current_snapshot) = model.accepted.as_ref().filter(|snapshot| {
                snapshot.token == action.owner.token
                    && snapshot.document.revision == action.owner.revision
            }) else {
                return;
            };
            let Some(board) = current_snapshot
                .document
                .boards
                .iter()
                .find(|board| board.id == action.owner.scope.board_id)
            else {
                return;
            };
            let Some(part) = current_snapshot
                .document
                .parts
                .iter()
                .find(|part| part.id == action.owner.part_id)
            else {
                return;
            };
            if !board.part_ids.contains(&part.id) || part.locked == Some(true) {
                return;
            }
            let mut position = part.pose.at;
            let baseline = match action.axis {
                pcb_wiring::PcbPositionAxis::X => {
                    let baseline = position.x;
                    position.x = action.value;
                    baseline
                }
                pcb_wiring::PcbPositionAxis::Y => {
                    let baseline = position.y;
                    position.y = action.value;
                    baseline
                }
            };
            if !action.value.is_finite() || baseline != action.baseline || position == part.pose.at
            {
                return;
            }
            let operation_id = runtime.operation();
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: action.owner.revision,
                    transaction_id: format!("pcb-part-position-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![part.id.clone()],
                    operation: EditOperation::MoveParts {
                        positions: vec![Position {
                            id: part.id.clone(),
                            at: position,
                        }],
                    },
                },
            });
        }
    };
    let case_admission = case_workspace::Admission {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        case_selection,
        instance_selection,
        owner_scope: render_scope.clone(),
        owner_token: snapshot.token,
        owner_generation: render_generation,
    };
    let on_case_action = {
        let admission = case_admission.clone();
        let navigate = workspace_callbacks.navigate;
        move |action| {
            if workspace() != "Case" {
                return;
            }
            case_workspace::apply_tree_action(action, &admission, navigate);
        }
    };
    let on_case_display = {
        let admission = case_admission;
        move |request| {
            if workspace() != "Case" {
                return;
            }
            case_workspace::apply_display_request(request, &admission);
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
            if !active_board_scope_matches(&model, &scope) {
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
    let on_keymap_export = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let scope = render_scope.clone();
        let token = snapshot.token;
        let revision = snapshot.document.revision;
        let generation = render_generation;
        move |_| {
            if workspace() != "Keymap"
                || runtime.scope().as_ref() != Some(&scope)
                || (adapter.generation)() != generation
            {
                return;
            }
            let model = runtime.model();
            if !active_board_scope_matches(&model, &scope) || !instance_selection.is_current(&model)
            {
                return;
            }
            if model.accepted.as_ref().is_none_or(|accepted| {
                accepted.token != token
                    || accepted.document.revision != revision
                    || accepted.scene.revision != revision
            }) {
                return;
            }
            runtime.export_firmware();
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
    let svg = use_hook(|| Rc::new(RefCell::new(None::<SvgElement>)));
    let workspace_rect_bounds = match active_workspace {
        "PCB" => pcb_bounds(snapshot, &render_scope),
        "Keymap" => keymap_view
            .as_deref()
            .and_then(|view| keymap_bounds(view, keymap_contours.as_deref().unwrap_or(&[]))),
        "Keycaps" => keycaps_view
            .as_deref()
            .and_then(|view| keycaps_bounds(view, keycaps_contours.as_deref().unwrap_or(&[]))),
        "Layout" => keycaps_fit::layout_canvas_bounds(
            &snapshot.document,
            &snapshot.scene,
            &render_scope.board_id,
        ),
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
    let bounds = bounds.unwrap_or((-50.0, 50.0, -50.0, 50.0));
    let bounds = if active_workspace == "Layout" {
        let surface = svg
            .borrow()
            .as_ref()
            .map(|surface| {
                let rect = surface.get_bounding_client_rect();
                (rect.width(), rect.height())
            })
            .unwrap_or((1.0, 1.0));
        keycaps_fit::aspect_bounds(bounds, surface)
    } else {
        let (min_x, max_x, min_y, max_y) = bounds;
        (min_x - 20.0, max_x + 20.0, min_y - 20.0, max_y + 20.0)
    };
    let (min_x, max_x, min_y, max_y) = bounds;
    let width = (max_x - min_x).max(50.0) / model.camera.zoom;
    let height = (max_y - min_y).max(50.0) / model.camera.zoom;
    let view_x = (min_x + max_x - width) * 0.5 + model.camera.center.x;
    let view_y = -(min_y + max_y + height) * 0.5 - model.camera.center.y;
    let view_box = format!("{view_x} {view_y} {width} {height}");
    let keymap_canvas_bounds = workspace_rect_bounds;
    let keymap_render_scope = render_scope.clone();
    let keymap_token = snapshot.token;
    let keymap_revision = snapshot.document.revision;
    let keymap_selected_ids = model.selected_part_ids.clone();
    let keymap_selected_set: BTreeSet<_> = keymap_selected_ids.iter().cloned().collect();
    let keymap_selection_available = keymap_view
        .as_deref()
        .is_some_and(|view| keymap::selected_bounds(view, &keymap_selected_set).is_some());
    let keymap_svg = svg.clone();
    let keymap_runtime = runtime.clone();
    let keymap_workspace = workspace;
    let board_view = keymap_view.clone();
    let board_contours = keymap_contours.clone();
    let on_fit_keymap_board = move |_| {
        let current = keymap_runtime.model();
        if keymap_workspace() != "Keymap"
            || current.active_board_id != keymap_render_scope.board_id
            || current.active_instance_id != keymap_render_scope.instance_id
        {
            return;
        }
        let Some(accepted) = current.accepted.as_ref() else {
            return;
        };
        if accepted.token != keymap_token
            || accepted.document.revision != keymap_revision
            || accepted.document.id != keymap_render_scope.document_id
            || accepted.session_epoch != keymap_render_scope.session_epoch
        {
            return;
        }
        let Some(view) = board_view.as_deref() else {
            return;
        };
        let Some(canvas_bounds) = keymap_canvas_bounds else {
            return;
        };
        let Some(target_bounds) = keymap_bounds(view, board_contours.as_deref().unwrap_or(&[]))
        else {
            return;
        };
        let svg_ref = keymap_svg.borrow();
        let Some(surface) = svg_ref.as_ref() else {
            return;
        };
        let rect = surface.get_bounding_client_rect();
        let Some(camera) = keymap::fit_camera(
            canvas_bounds,
            target_bounds,
            (rect.width(), rect.height()),
            keymap_toolbar_height(),
        ) else {
            return;
        };
        keymap_runtime.submit(Event::SetCamera {
            operation_id: keymap_runtime.operation(),
            center: camera.center,
            zoom: camera.zoom,
        });
    };
    let selection_runtime = runtime.clone();
    let selection_workspace = workspace;
    let selection_scope = render_scope.clone();
    let selection_svg = svg.clone();
    let selection_view = keymap_view.clone();
    let selection_canvas_bounds = keymap_canvas_bounds;
    let selection_token = snapshot.token;
    let selection_revision = snapshot.document.revision;
    let selection_ids = keymap_selected_ids.clone();
    let on_fit_keymap_selection = move |_| {
        let current = selection_runtime.model();
        if selection_workspace() != "Keymap"
            || current.active_board_id != selection_scope.board_id
            || current.active_instance_id != selection_scope.instance_id
            || current.selected_part_ids != selection_ids
        {
            return;
        }
        let Some(accepted) = current.accepted.as_ref() else {
            return;
        };
        if accepted.token != selection_token
            || accepted.document.revision != selection_revision
            || accepted.document.id != selection_scope.document_id
            || accepted.session_epoch != selection_scope.session_epoch
        {
            return;
        }
        let Some(view) = selection_view.as_deref() else {
            return;
        };
        let selected = selection_ids.iter().cloned().collect();
        let Some(target_bounds) = keymap::selected_bounds(view, &selected) else {
            return;
        };
        let Some(canvas_bounds) = selection_canvas_bounds else {
            return;
        };
        let svg_ref = selection_svg.borrow();
        let Some(surface) = svg_ref.as_ref() else {
            return;
        };
        let rect = surface.get_bounding_client_rect();
        let Some(camera) = keymap::fit_camera(
            canvas_bounds,
            target_bounds,
            (rect.width(), rect.height()),
            keymap_toolbar_height(),
        ) else {
            return;
        };
        selection_runtime.submit(Event::SetCamera {
            operation_id: selection_runtime.operation(),
            center: camera.center,
            zoom: camera.zoom,
        });
    };
    let canvas_center =
        part_placement::canvas_world_center(min_x, max_x, min_y, max_y, model.camera.center);
    let part_placement =
        part_placement::use_controller_placement(part_placement::PartPlacementHost {
            runtime: part_placement::runtime_adapter(runtime.clone()),
            load_definition: Rc::new(|document, definition_id| {
                Box::pin(async move {
                    parts::load_component_definition(&document, &definition_id).await
                })
            }),
            workspace,
            generation: adapter.generation,
            version,
            adapter: adapter.clone(),
            layout_selection_kind,
            guide_preferences,
            parts_query,
            parts_selection,
            snap_settings: layout_snap_settings,
            layout_target,
            canvas_center,
            objects_open,
            inspect_open,
            canvas_interaction: canvas_interaction.clone(),
        });
    if canvas_interaction.current().is_some() {
        matrix_setup.can_open = false;
    }
    let pair_placement_owner = mirrored_pair
        .placement
        .as_ref()
        .map(|placement| placement.owner.clone());
    use_effect(use_reactive((&pair_placement_owner,), {
        let svg = svg.clone();
        move |(owner,)| {
            if owner.is_some()
                && let Some(surface) = svg.borrow().as_ref()
            {
                let options = web_sys::FocusOptions::new();
                options.set_prevent_scroll(true);
                let _ = surface.focus_with_options(&options);
            }
        }
    }));
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
    let on_keycaps_finding = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let scope = render_scope.clone();
        let generation = render_generation;
        let mut workspace = workspace;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        let mut body_selection = case_body_selection;
        let case_selection_for_owner = case_selection;
        let navigation_alive = keycaps_navigation_alive.clone();
        let inspector_settings = inspector_panel_settings;
        let fit_state = keycaps_fit_state.state.clone();
        let mut pending_camera_fit = pending_keycaps_navigation_fit;
        let mut focused_finding = focused_keycaps_finding;
        let source_surface = svg.clone();
        let select_tree = workspace_callbacks.select_tree;
        move |request: keycaps_fit::FindingNavigationRequest| {
            let owner = keycaps_navigation::OwnerIdentity {
                scope: &scope,
                generation,
            };
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let live_mechanical_layers = current_case_layer_ids(&runtime, &scope, snapshot);
            let Some(state) = fit_state.as_ref() else {
                return;
            };
            let Some(accepted_scope) = runtime.scope() else {
                return;
            };
            let accepted = keycaps_navigation::AcceptedNavigationSource {
                scope: accepted_scope,
                session_epoch: snapshot.session_epoch,
                token: snapshot.token,
                revision: snapshot.document.revision,
                active_board_id: model.active_board_id.clone(),
            };
            let Some(admitted) = keycaps_navigation::admit_accepted_request(
                &request,
                keycaps_navigation::NavigationAdmission {
                    current_workspace: workspace(),
                    owner,
                    live_scope: runtime.scope().as_ref(),
                    live_generation: (adapter.generation)(),
                    accepted: &accepted,
                    fit_state: state,
                    document: &snapshot.document,
                    live_mechanical_layers: live_mechanical_layers.as_deref(),
                },
                |target| match target {
                    keycaps_fit::FindingNavigationTarget::Part { part_id, .. } => {
                        objects::component_context_for_finding_part(&model, part_id)
                    }
                    _ => None,
                },
            ) else {
                return;
            };
            let focused =
                keycaps_navigation::focused_finding_for_admitted_route(&request, &admitted);
            let effects = admitted.effects;
            let navigation_owner = admitted.owner;
            let source_basis = if workspace() == "Keycaps" {
                source_surface.borrow().as_ref().and_then(|surface| {
                    let rect = surface.get_bounding_client_rect();
                    keycaps_fit::layout_camera_basis(
                        &snapshot.document,
                        &snapshot.scene,
                        &request.source.scope.board_id,
                        (rect.width(), rect.height()),
                    )
                })
            } else {
                None
            };
            focused_finding.set(focused);
            let focus_runtime = runtime.clone();
            let focus_adapter = adapter.clone();
            let focus_body_selection = body_selection;
            let focus_case_selection = case_selection_for_owner;
            let focus_workspace = workspace;
            pending_camera_fit.set(None);
            keycaps_navigation::dispatch_route_with_camera_basis(
                effects,
                &request,
                source_basis,
                |effect| match effect {
                    keycaps_navigation::RouteAction::SetWorkspace(name) => workspace.set(name),
                    keycaps_navigation::RouteAction::SelectTree { scope, context } => {
                        select_tree.call(objects::TreeSelectRequest {
                            scope,
                            context,
                            mode: SelectionMode::Replace,
                            outline_action: None,
                        });
                    }
                    keycaps_navigation::RouteAction::SelectCaseLayer { scope, layer_id } => {
                        case_selection.select_layer(scope, layer_id);
                    }
                    keycaps_navigation::RouteAction::SelectCaseBody { scope, body_id } => {
                        // Dioxus retains body and layer selections independently; clear an older
                        // layer so the body finding becomes the active, reachable Inspector owner.
                        case_selection_for_owner.clear_layer_for_scope(&scope);
                        body_selection.set(Some(case_viewer::BodySelection { scope, body_id }));
                    }
                    keycaps_navigation::RouteAction::CloseObjects => objects_open.set(false),
                    keycaps_navigation::RouteAction::OpenInspector => inspect_open.set(true),
                    keycaps_navigation::RouteAction::PinInspector => {
                        pin_inspector_on_desktop(inspector_settings);
                    }
                    keycaps_navigation::RouteAction::QueueLayoutFit(request) => {
                        pending_camera_fit.set(Some(*request));
                    }
                    keycaps_navigation::RouteAction::Report(message) => runtime.report(message),
                    keycaps_navigation::RouteAction::FocusInspector => {
                        let scheduled_runtime = focus_runtime.clone();
                        let scheduled_adapter = focus_adapter.clone();
                        focus_first_inspector_control_on_next_frame(
                            navigation_alive.clone(),
                            navigation_owner.clone(),
                            move || {
                                current_keycaps_navigation_owner(
                                    focus_workspace,
                                    &scheduled_runtime,
                                    &scheduled_adapter,
                                    focus_body_selection,
                                    focus_case_selection,
                                )
                            },
                        );
                    }
                },
            );
        }
    };
    let observed_navigation_selection = (adapter.selected_context)();
    let observed_navigation_generation = (adapter.generation)();
    keycaps_navigation::use_pending_layout_fit(
        pending_keycaps_navigation_fit,
        (
            workspace(),
            observed_navigation_selection,
            observed_navigation_generation,
            version(),
        ),
        keycaps_navigation_alive.clone(),
        {
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let body_selection = case_body_selection;
            move || {
                current_keycaps_navigation_owner(
                    workspace,
                    &runtime,
                    &adapter,
                    body_selection,
                    case_selection,
                )
            }
        },
        {
            let runtime = runtime.clone();
            let state = keycaps_fit_state.state.clone();
            let surface = svg.clone();
            move |pending_fit| {
                let request = &pending_fit.request;
                let model = runtime.model();
                let snapshot = model.accepted.as_ref().filter(|snapshot| {
                    snapshot.token == request.source.token
                        && snapshot.document.id == request.source.scope.document_id
                        && snapshot.document.revision == request.source.revision
                        && snapshot.session_epoch == request.source.scope.session_epoch
                        && model.active_board_id == request.source.scope.board_id
                })?;
                let live_layers =
                    current_case_layer_ids(&runtime, &pending_fit.owner.scope, snapshot);
                if live_layers.is_none() && request.source.case_preview_current {
                    return None;
                }
                let live_layers = live_layers.unwrap_or_else(|| Rc::from([]));
                let state = state.as_ref()?;
                if keycaps_fit::accepted_navigation_target(
                    state,
                    request,
                    &snapshot.document,
                    &live_layers,
                ) != Some(request.target.clone())
                {
                    return None;
                }
                let target_bounds = keycaps_fit::finding_navigation_bounds(
                    &snapshot.document,
                    &snapshot.scene,
                    &request.finding.id,
                    &request.target,
                )?;
                let surface = surface.borrow().clone()?;
                let rect = surface.get_bounding_client_rect();
                let destination_surface = (rect.width(), rect.height());
                let basis = if let Some(basis) = pending_fit.source_basis {
                    basis
                } else {
                    let destination_base = keycaps_fit::layout_canvas_bounds(
                        &snapshot.document,
                        &snapshot.scene,
                        &request.source.scope.board_id,
                    )?;
                    keycaps_fit::CameraBasis {
                        bounds: keycaps_fit::aspect_bounds(destination_base, destination_surface),
                        surface: destination_surface,
                    }
                };
                Some(keycaps_navigation::DestinationFitGeometry {
                    base: basis.bounds,
                    target: target_bounds,
                    surface: basis.surface,
                })
            }
        },
        {
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let body_selection = case_body_selection;
            let alive = keycaps_navigation_alive.clone();
            move |action, expected| match action {
                keycaps_navigation::FitAction::SetCamera(camera_fit) => {
                    runtime.submit(Event::SetCamera {
                        operation_id: runtime.operation(),
                        center: camera_fit.center,
                        zoom: camera_fit.zoom,
                    });
                }
                keycaps_navigation::FitAction::FocusInspector => {
                    let focus_runtime = runtime.clone();
                    let focus_adapter = adapter.clone();
                    focus_first_inspector_control_on_next_frame(
                        alive.clone(),
                        expected,
                        move || {
                            current_keycaps_navigation_owner(
                                workspace,
                                &focus_runtime,
                                &focus_adapter,
                                body_selection,
                                case_selection,
                            )
                        },
                    );
                }
            }
        },
    );
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
    let component_inspector_key = layout_component_inspector_owner_key(
        &model,
        active_workspace,
        selected_tree_context.as_ref(),
    );
    let component_inspector_generation =
        layout_component_inspector_lifetime.update(component_inspector_key);
    let component_inspector = (active_workspace == "Layout")
        .then(|| {
            layout_component_inspector_projection(
                &model,
                selected_tree_context.as_ref(),
                component_inspector_generation,
                (adapter.generation)(),
            )
        })
        .flatten();
    let show_position_inspector = component_inspector.is_none()
        && selected_tree_context.as_ref().is_none_or(|selected| {
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
        let focus_placement = part_placement.projection.is_some();
        move |event: MountedEvent| {
            if let Some(element) = event
                .data()
                .try_as_web_event()
                .and_then(|e| e.dyn_into::<SvgElement>().ok())
            {
                runtime.surface(element.clone());
                if focus_placement {
                    let options = web_sys::FocusOptions::new();
                    options.set_prevent_scroll(true);
                    let _ = element.focus_with_options(&options);
                }
                *svg.borrow_mut() = Some(element);
            }
        }
    };
    let placement_active = part_placement.projection.is_some();
    let focus_svg = svg.clone();
    use_effect(use_reactive!(|placement_active| {
        if !placement_active {
            return;
        }
        if let Some(element) = focus_svg.borrow().as_ref() {
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = element.focus_with_options(&options);
        }
    }));
    let move_pointer = {
        let runtime = runtime.clone();
        let svg = svg.clone();
        let drag = drag.clone();
        let adapter = adapter.clone();
        let render_scope = render_scope.clone();
        let owner = layout_owner.clone();
        let snap_settings = layout_snap_settings;
        let placement = part_placement.clone();
        let tree_cell_anchor = tree_cell_anchor.clone();
        let mirrored_pair = mirrored_pair.clone();
        let canvas_interaction = canvas_interaction.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair) {
                let Some(placement) = mirrored_pair.placement.as_ref() else {
                    pointer.prevent_default();
                    pointer.stop_propagation();
                    return;
                };
                let pair_owner = placement.owner.clone();
                if workspace() != "Layout"
                    || runtime.scope().as_ref() != Some(&pair_owner.scope)
                    || (adapter.generation)() != pair_owner.scope_generation
                {
                    mirrored_pair.on_cancel.call(pair_owner);
                    return;
                }
                if let Some(center) = coordinates(&svg, &pointer, view_x, view_y, width, height) {
                    mirrored_pair.on_move.call(objects::MirroredPairMove {
                        owner: pair_owner,
                        center,
                    });
                }
                return;
            }
            if canvas_interaction.is_owner(CanvasInteractionOwner::PartPlacement) {
                if let Some(active) = placement.projection.clone()
                    && let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height)
                {
                    let model = runtime.model();
                    let at = model.accepted.as_ref().map_or(point, |_snapshot| {
                        part_placement::snap_placement_at(
                            &active.snap_document,
                            &active.owner.board_id,
                            &active.pending,
                            point,
                            {
                                let settings = snap_settings.read();
                                part_placement::PlacementSnapOptions {
                                    snap_fraction: settings.snap_fraction,
                                    geometry_snap: settings.geometry_snap,
                                    gap: part_placement::placement_gap(&settings),
                                    free: pointer.alt_key(),
                                }
                            },
                        )
                    });
                    placement.on_move.call(at);
                } else {
                    pointer.prevent_default();
                    pointer.stop_propagation();
                }
                return;
            }
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
            if !canvas_owner_is_current(&runtime, workspace, &adapter, &owner) {
                selection::cancel_drag_if_owned(&runtime, &current);
                drag.borrow_mut().take();
                if let Some(element) = svg.borrow().as_ref() {
                    let _ = element.release_pointer_capture(pointer.pointer_id());
                }
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
                if !canvas_owner_is_current(&runtime, workspace, &adapter, &owner)
                    || owner.scope.as_ref() != Some(&current.scope)
                {
                    drag.borrow_mut().take();
                    if let Some(element) = svg.borrow().as_ref() {
                        let _ = element.release_pointer_capture(pointer.pointer_id());
                    }
                    return;
                }
                let current_model = runtime.model();
                let retained_cell = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
                let (matrix_pitch, matrix_gap) = matrix_snap_parameters(
                    &current_model,
                    &current.scope,
                    &adapter,
                    retained_cell.as_ref(),
                );
                let settings = snap_settings.read().clone();
                let snap = objects::gesture_snap_inputs(&settings, matrix_pitch, matrix_gap);
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
                    pitch: snap.pitch,
                    snap_fraction: snap.snap_fraction,
                    geometry_snap: snap.geometry_snap,
                    gap: snap.gap,
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
        let owner = layout_owner.clone();
        let placement = part_placement.clone();
        let snap_settings = layout_snap_settings;
        let render_scope = render_scope.clone();
        let canvas_interaction = canvas_interaction.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair) {
                pointer.prevent_default();
                pointer.stop_propagation();
                return;
            }
            if canvas_interaction.is_owner(CanvasInteractionOwner::PartPlacement) {
                pointer.prevent_default();
                pointer.stop_propagation();
                if placement.projection.is_none()
                    || !part_placement::pointer_release_commits(pointer.button())
                {
                    return;
                }
                if let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height) {
                    let at = runtime
                        .model()
                        .accepted
                        .as_ref()
                        .map_or(point, |_snapshot| {
                            placement.projection.as_ref().map_or(point, |active| {
                                part_placement::snap_placement_at(
                                    &active.snap_document,
                                    &active.owner.board_id,
                                    &active.pending,
                                    point,
                                    {
                                        let settings = snap_settings.read();
                                        part_placement::PlacementSnapOptions {
                                            snap_fraction: settings.snap_fraction,
                                            geometry_snap: settings.geometry_snap,
                                            gap: part_placement::placement_gap(&settings),
                                            free: pointer.alt_key(),
                                        }
                                    },
                                )
                            })
                        });
                    placement.on_commit.call(at);
                }
                return;
            }
            let Some(current) = drag
                .borrow()
                .clone()
                .filter(|d| d.pointer == i64::from(pointer.pointer_id()))
            else {
                return;
            };
            if !canvas_owner_is_current(&runtime, workspace, &adapter, &owner) {
                selection::cancel_drag_if_owned(&runtime, &current);
                drag.borrow_mut().take();
                if let Some(element) = svg.borrow().as_ref() {
                    let _ = element.release_pointer_capture(pointer.pointer_id());
                }
                return;
            }
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
        let placement = part_placement.clone();
        let render_scope = render_scope.clone();
        let canvas_interaction = canvas_interaction.clone();
        move |event: PointerEvent| {
            match canvas_interaction.current() {
                Some(CanvasInteractionOwner::MirroredPair) => return,
                Some(CanvasInteractionOwner::PartPlacement) => {
                    if placement.projection.is_some() {
                        placement.on_cancel.call(());
                    }
                    return;
                }
                Some(CanvasInteractionOwner::OutlinePerimeter) => return,
                None => {}
            }
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
        move |_| {
            space_down.set(false);
            selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&render_scope));
            let mut interaction_version = interaction_version;
            interaction_version += 1;
        }
    }));
    let keyboard = {
        let runtime = runtime.clone();
        let drag = drag.clone();
        let svg = svg.clone();
        let render_scope = render_scope.clone();
        let space_down = space_down.clone();
        let mirrored_pair = mirrored_pair.clone();
        let placement = part_placement.clone();
        let canvas_interaction = canvas_interaction.clone();
        let snap_settings = layout_snap_settings;
        move |event: KeyboardEvent| {
            let key = event.data().key().to_string();
            let code = event.data().code().to_string();
            let modifiers = event.data().modifiers();
            if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair)
                && let Some(placement) = mirrored_pair.placement.as_ref()
            {
                let owner = placement.owner.clone();
                if key == "Escape" {
                    event.prevent_default();
                    mirrored_pair.on_cancel.call(owner);
                    return;
                }
                if key == "Enter" {
                    event.prevent_default();
                    mirrored_pair.on_commit.call(objects::MirroredPairMove {
                        owner,
                        center: placement.pair.center,
                    });
                    return;
                }
                let (dx, dy) = match key.as_str() {
                    "ArrowLeft" => (-1.0, 0.0),
                    "ArrowRight" => (1.0, 0.0),
                    "ArrowDown" => (0.0, -1.0),
                    "ArrowUp" => (0.0, 1.0),
                    _ => (0.0, 0.0),
                };
                if dx != 0.0 || dy != 0.0 {
                    event.prevent_default();
                    let settings = snap_settings.read();
                    let fraction = settings.snap_fraction;
                    let pitch = placement.pair.matrix.pitch;
                    let step = |axis_pitch: f64| {
                        if fraction.is_sign_negative() {
                            -fraction
                        } else {
                            axis_pitch * if fraction == 0.0 { 0.25 } else { fraction }
                        }
                    };
                    mirrored_pair.on_move.call(objects::MirroredPairMove {
                        owner,
                        center: Vec2 {
                            x: placement.pair.center.x + dx * step(pitch.x),
                            y: placement.pair.center.y + dy * step(pitch.y),
                        },
                    });
                    return;
                }
            }
            if key == "Escape"
                && canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair)
                && let Some(form) = mirrored_pair.form.as_ref()
                && mirrored_pair.placement.is_none()
            {
                event.prevent_default();
                mirrored_pair.on_cancel.call(form.owner.clone());
                return;
            }
            if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair) {
                return;
            }
            if canvas_interaction.is_owner(CanvasInteractionOwner::PartPlacement) {
                if let Some(active) = placement.projection.as_ref() {
                    if key == "Escape" {
                        event.prevent_default();
                        placement.on_cancel.call(());
                    } else if key == "Enter" {
                        event.prevent_default();
                        placement.on_commit.call(active.pending.at);
                    } else if matches!(
                        key.as_str(),
                        "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight"
                    ) {
                        event.prevent_default();
                        let mut at = active.pending.at;
                        let fraction = snap_settings.read().snap_fraction;
                        let step = if fraction < 0.0 {
                            -fraction
                        } else if fraction > 0.0 {
                            19.05 * fraction
                        } else {
                            0.1
                        };
                        match key.as_str() {
                            "ArrowUp" => at.y += step,
                            "ArrowDown" => at.y -= step,
                            "ArrowLeft" => at.x -= step,
                            "ArrowRight" => at.x += step,
                            _ => {}
                        }
                        placement.on_move.call(at);
                    }
                } else if key == "Escape" {
                    event.prevent_default();
                    placement.on_cancel.call(());
                } else {
                    event.prevent_default();
                }
                return;
            }
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
        let mirrored_pair = mirrored_pair.clone();
        let canvas_interaction = canvas_interaction.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair)
                && let Some(placement) = mirrored_pair.placement.as_ref()
                && pointer.button() == 0
            {
                let owner = placement.owner.clone();
                if workspace() != "Layout"
                    || runtime.scope().as_ref() != Some(&owner.scope)
                    || (adapter.generation)() != owner.scope_generation
                {
                    mirrored_pair.on_cancel.call(owner);
                    return;
                }
                if let Some(center) = coordinates(&svg, &pointer, view_x, view_y, width, height) {
                    pointer.prevent_default();
                    pointer.stop_propagation();
                    mirrored_pair
                        .on_commit
                        .call(objects::MirroredPairMove { owner, center });
                }
                return;
            }
            if canvas_interaction.current().is_some() {
                pointer.prevent_default();
                pointer.stop_propagation();
                return;
            }
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
    let on_pcb_part_pointer_down = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let svg = svg.clone();
        let drag = drag.clone();
        let owner = layout_owner.clone();
        let scope = render_scope.clone();
        let accepted_token = snapshot.token;
        let accepted_revision = snapshot.document.revision;
        let generation = render_generation;
        let space_down = space_down.clone();
        let tree_cell_anchor = tree_cell_anchor.clone();
        let canvas_interaction = canvas_interaction.clone();
        move |request: pcb_scene::PcbPartPointerDown| {
            if request.scope != scope
                || request.token != accepted_token
                || request.generation != generation
                || !pcb_owner_is_current(&runtime, workspace, &adapter, &owner)
                || !instance_selection.is_current(&runtime.model())
                || canvas_interaction.current().is_some()
                || drag.borrow().is_some()
                || runtime.model().gesture.is_some()
            {
                return;
            }
            let Some(origin) = coordinates_at(
                &svg,
                request.client_x,
                request.client_y,
                view_x,
                view_y,
                width,
                height,
            ) else {
                return;
            };
            if space_down.get() {
                if let Some(surface) = svg.borrow().as_ref() {
                    let _ = surface.set_pointer_capture(request.pointer_id as i32);
                    let options = web_sys::FocusOptions::new();
                    options.set_prevent_scroll(true);
                    let _ = surface.focus_with_options(&options);
                }
                *drag.borrow_mut() = Some(Drag {
                    pointer: request.pointer_id,
                    scope: scope.clone(),
                    generation,
                    gesture_generation: None,
                    origin,
                    client_x: f64::from(request.client_x),
                    client_y: f64::from(request.client_y),
                    positions: vec![],
                    active: true,
                    pan: true,
                    camera: runtime.model().camera.center,
                });
                return;
            }
            let model = runtime.model();
            let Some(board) = model.accepted.as_ref().and_then(|snapshot| {
                snapshot
                    .document
                    .boards
                    .iter()
                    .find(|board| board.id == scope.board_id)
            }) else {
                return;
            };
            if !board.part_ids.contains(&request.part_id) {
                return;
            }
            let Some(hit_context) = objects::context_for_part(&model, &request.part_id) else {
                return;
            };
            if !selection::context_is_current(&model, &scope, &hit_context) {
                return;
            }
            update_tree_cell_anchor(&tree_cell_anchor, &owner, &model, Some(&hit_context));
            let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
            let projection = objects::context_for_selection_kind(
                &model,
                &hit_context,
                layout_selection_kind(),
                retained.as_ref(),
            );
            let context = projection
                .as_ref()
                .map(|projection| projection.context.clone())
                .unwrap_or(hit_context);
            let mode = if request.range {
                SelectionMode::Range
            } else if request.additive {
                SelectionMode::Toggle
            } else {
                SelectionMode::Replace
            };
            let range_ids = if mode == SelectionMode::Range {
                selection::eligible_live_ids(&model)
                    .into_iter()
                    .filter(|id| board.part_ids.contains(id))
                    .collect()
            } else {
                Vec::new()
            };
            if !model.selected_part_ids.contains(&request.part_id)
                || mode != SelectionMode::Replace
                || projection
                    .as_ref()
                    .is_some_and(|projection| projection.part_ids != model.selected_part_ids)
            {
                selection::submit_canvas_selection(
                    &runtime,
                    &adapter,
                    &scope,
                    generation,
                    context.clone(),
                    mode,
                    range_ids,
                );
            } else {
                let mut selected_context = adapter.selected_context;
                selected_context.set(Some(objects::ScopedTreeContext {
                    scope: scope.clone(),
                    context: context.clone(),
                }));
            }
            if runtime.scope().as_ref() != Some(&scope) || (adapter.generation)() != generation {
                return;
            }
            let current = runtime.model();
            let Some(current_snapshot) = current.accepted.as_ref() else {
                return;
            };
            if current_snapshot.token != request.token
                || current_snapshot.document.revision != accepted_revision
                || !current.selected_part_ids.contains(&request.part_id)
            {
                return;
            }
            let positions: Vec<_> = current_snapshot
                .document
                .parts
                .iter()
                .filter(|part| {
                    current.selected_part_ids.contains(&part.id)
                        && board.part_ids.contains(&part.id)
                        && part.locked != Some(true)
                })
                .map(|part| Position {
                    id: part.id.clone(),
                    at: part.pose.at,
                })
                .collect();
            if positions.is_empty()
                || positions.len() != current.selected_part_ids.len()
                || current.lifecycle != Lifecycle::Ready
                || current.durability
                    != (Durability::Saved {
                        revision: current_snapshot.document.revision,
                    })
                || current.display_preview.is_some()
                || current.gesture.is_some()
            {
                return;
            }
            if let Some(surface) = svg.borrow().as_ref() {
                let _ = surface.set_pointer_capture(request.pointer_id as i32);
                let options = web_sys::FocusOptions::new();
                options.set_prevent_scroll(true);
                let _ = surface.focus_with_options(&options);
            }
            *drag.borrow_mut() = Some(Drag {
                pointer: request.pointer_id,
                scope: scope.clone(),
                generation,
                gesture_generation: None,
                origin,
                client_x: f64::from(request.client_x),
                client_y: f64::from(request.client_y),
                positions,
                active: false,
                pan: false,
                camera: Vec2::default(),
            });
        }
    };
    let on_pcb_empty_hit = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let scope = render_scope.clone();
        let generation = render_generation;
        let token = snapshot.token;
        let space_down = space_down.clone();
        let drag = drag.clone();
        let canvas_interaction = canvas_interaction.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            if pointer.button() != 0
                || space_down.get()
                || drag.borrow().is_some()
                || canvas_interaction.current().is_some()
                || workspace() != "PCB"
                || runtime.scope().as_ref() != Some(&scope)
                || (adapter.generation)() != generation
            {
                return;
            }
            let model = runtime.model();
            if !active_board_scope_matches(&model, &scope) || !instance_selection.is_current(&model)
            {
                return;
            }
            let Some(snapshot) = model
                .accepted
                .as_ref()
                .filter(|snapshot| snapshot.token == token)
            else {
                return;
            };
            if !snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == scope.board_id)
            {
                return;
            }
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
        .keycaps_finding
        .replace(Box::new(on_keycaps_finding));
    workspace_callbacks
        .pcb_empty_hit
        .replace(Box::new(on_pcb_empty_hit));
    workspace_callbacks
        .pcb_part_hit
        .replace(Box::new(on_pcb_part_hit));
    workspace_callbacks
        .pcb_part_pointer_down
        .replace(Box::new(on_pcb_part_pointer_down));
    workspace_callbacks
        .pcb_wiring_edit_board
        .replace(Box::new(on_pcb_wiring_edit_board));
    workspace_callbacks
        .layout_selection_kind
        .replace(Box::new(on_layout_selection_kind));
    workspace_callbacks
        .layout_snap_intent
        .replace(Box::new(on_layout_snap_intent));
    workspace_callbacks
        .pcb_selection_kind
        .replace(Box::new(on_pcb_selection_kind));
    workspace_callbacks
        .pcb_snap_intent
        .replace(Box::new(on_pcb_snap_intent));
    workspace_callbacks
        .pcb_part_position
        .replace(Box::new(on_pcb_part_position));
    workspace_callbacks
        .case_action
        .replace(Box::new(on_case_action));
    workspace_callbacks
        .case_display
        .replace(Box::new(on_case_display));
    workspace_callbacks
        .keymap_layer
        .replace(Box::new(on_keymap_layer));
    workspace_callbacks
        .keymap_export
        .replace(Box::new(on_keymap_export));
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
    let case_scene = case_workspace::workspace_display_scene(runtime.cad_scene(), &render_scope);
    let pcb_wiring_source = current_scope
        .as_ref()
        .filter(|scope| {
            active_board_scope_matches(&model, scope) && instance_selection.is_current(&model)
        })
        .and_then(|scope| {
            let mut board_scope = scope.clone();
            board_scope.instance_id = None;
            pcb_wiring::PcbWiringSource::new(
                snapshot,
                &board_scope,
                scope,
                model.selected_part_ids.first().map(String::as_str),
                runtime.electrical_preview_executor_epoch(),
                (adapter.generation)(),
            )
        });
    let pcb_wiring_mode_actions = pcb_wiring::use_board_wiring_mode_edits(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
        pcb_wiring_mount.resolution_signal,
    );
    let pcb_wiring_pin_actions = pcb_wiring::use_pcb_wiring_pin_edits(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
        pcb_wiring_mount.resolution_signal,
    );
    let pcb_wiring_protected_remap_actions = pcb_wiring::use_protected_remap_review(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
    );
    let pcb_wiring_apply_actions = pcb_wiring::use_board_wiring_apply(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
        pcb_wiring_mount.resolution_signal,
    );
    let zmk_firmware_export_panel = use_export_panel_input(
        runtime.clone(),
        workspace,
        adapter.generation,
        pcb_wiring_source.clone(),
        pcb_wiring_mount.resolution_signal,
        instance_selection,
    );
    let case_selected_body_id = case_selection
        .body
        .read()
        .as_ref()
        .filter(|selection| selection.scope == render_scope)
        .map(|selection| selection.body_id.clone());
    let case_selected_layer_id = case_selection.layer_id(&render_scope);
    let case_selected_context = adapter
        .selected_context
        .read()
        .clone()
        .filter(|selected| selected.scope == render_scope);
    let on_browse_parts = EventHandler::new(move |_| {
        browse_parts_workspace(
            workspace,
            objects_open,
            objects_panel_settings,
            is_compact_viewport(),
        );
    });
    let objects_input = match active_workspace {
        "PCB" => workspace_composition::WorkspaceObjectsInput::Pcb(shared_objects),
        "Keymap" => workspace_composition::WorkspaceObjectsInput::Keymap(shared_objects),
        "Keycaps" => workspace_composition::WorkspaceObjectsInput::Keycaps(shared_objects),
        "Case" => workspace_composition::WorkspaceObjectsInput::Case(Box::new(
            case_workspace::ObjectsInput {
                model: &model,
                scope: current_scope.clone(),
                instance_scope_pending,
                scene: case_scene.clone(),
                selected_context: adapter.selected_context,
                selected_body_id: case_selected_body_id.clone(),
                selected_layer_id: case_selected_layer_id.clone(),
                case_selection,
                expanded: case_tree_expanded,
                on_action: workspace_callbacks.case_action,
                on_select: workspace_callbacks.select_tree,
                on_navigate: workspace_callbacks.navigate,
                on_display: workspace_callbacks.case_display,
            },
        )),
        "Parts" => {
            workspace_composition::WorkspaceObjectsInput::Parts(parts_workspace::ObjectsInput {
                snapshot: snapshot.clone(),
                scope: current_scope.clone(),
                scope_generation: adapter.generation,
                workspace,
                query: parts_query,
                selected: parts_selection,
                on_select: workspace_callbacks.parts_select,
            })
        }
        _ => workspace_composition::WorkspaceObjectsInput::Layout(Box::new(
            layout_workspace::ObjectsInput {
                shared: shared_objects,
                matrix_setup: matrix_setup.clone(),
                mirrored_pair: mirrored_pair.clone(),
                pair_created: pair_created_selection,
                on_place_component: part_placement.on_place_component,
                layout_target,
                parts_query,
                on_browse_parts,
                placement_error: part_placement.error.clone(),
            },
        )),
    };
    let on_layout_view_mode = layout_view_mode_handler(
        {
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let owner = layout_owner.clone();
            move || layout_owner_is_current(&runtime, workspace, &adapter, &owner)
        },
        layout_assembly_3d,
        move |assembly_3d| layout_assembly_3d.set(assembly_3d),
        part_placement.clone(),
        canvas_interaction.clone(),
        {
            let runtime = runtime.clone();
            let drag = drag.clone();
            let svg = svg.clone();
            let render_scope = render_scope.clone();
            move || {
                selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&render_scope));
                let mut interaction_version = interaction_version;
                interaction_version += 1;
            }
        },
        {
            let mirrored_pair = mirrored_pair.clone();
            let matrix_setup = matrix_setup.clone();
            move || {
                if let Some(active) = mirrored_pair.placement.as_ref() {
                    mirrored_pair.on_cancel.call(active.owner.clone());
                } else if let Some(form) = mirrored_pair.form.as_ref() {
                    mirrored_pair.on_cancel.call(form.owner.clone());
                }
                if let Some(projection) = matrix_setup.projection.as_ref() {
                    matrix_setup.on_cancel.call(projection.owner.clone());
                }
            }
        },
    );
    let toolbar_input = match active_workspace {
        "Layout" => {
            let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &layout_owner);
            let supports_kind = |kind| {
                selected_tree_context.as_ref().is_some_and(|selected| {
                    objects::context_for_selection_kind(
                        &model,
                        &selected.context,
                        kind,
                        retained.as_ref(),
                    )
                    .is_some()
                })
            };
            let properties_available = selected_tree_context.as_ref().is_some_and(|selected| {
                matches!(
                    &selected.context,
                    objects::TreeContext::Matrix { .. }
                        | objects::TreeContext::Row { .. }
                        | objects::TreeContext::Column { .. }
                        | objects::TreeContext::Key { .. }
                        | objects::TreeContext::Component { .. }
                )
            }) && (show_position_inspector
                || matrix_transform_inspector.projection.is_some());
            let on_show_properties = EventHandler::new({
                let runtime = runtime.clone();
                let adapter = adapter.clone();
                let owner = layout_owner.clone();
                let mut objects_open = objects_open;
                let mut inspect_open = inspect_open;
                move |_| {
                    if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
                        return;
                    }
                    if !properties_available {
                        return;
                    }
                    pin_inspector_on_desktop(inspector_panel_settings);
                    objects_open.set(false);
                    inspect_open.set(true);
                }
            });
            let transform = objects::LayoutTransformMenuMount {
                properties_available,
                column_available: supports_kind(objects::LayoutSelectionKind::Column),
                row_available: supports_kind(objects::LayoutSelectionKind::Row),
                on_selection_kind: workspace_callbacks.layout_selection_kind,
                on_show_properties,
            };
            let footprints_pressed = (layer_visibility.footprints)()
                && !(layer_visibility.hidden)().contains("Footprints");
            workspace_composition::WorkspaceToolbarInput::Layout(Box::new(
                layout_workspace::ToolbarInput {
                    save_failure: match &model.durability {
                        Durability::Failed { reason, .. } => Some(reason.clone()),
                        _ => None,
                    },
                    recovery_required: model.lifecycle
                        == boardstudio_application::Lifecycle::RecoveryRequired,
                    footprints_pressed,
                    on_toggle_footprints: workspace_callbacks.toggle_footprints,
                    assembly_3d: layout_assembly_3d(),
                    on_view_mode: on_layout_view_mode,
                    on_retry_save: workspace_callbacks.retry_save,
                    on_recover_saved: workspace_callbacks.recover_saved,
                    selection_kind: layout_selection_kind(),
                    snap_settings: layout_snap_settings.read().clone(),
                    command_label: "Layout commands".to_owned(),
                    align: layout_align.clone(),
                    transform,
                    menu_owner_key: format!("{layout_owner:?}"),
                    on_selection_kind: workspace_callbacks.layout_selection_kind,
                    on_snap_intent: workspace_callbacks.layout_snap_intent,
                },
            ))
        }
        "PCB" => {
            let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &layout_owner);
            let supports_kind = |kind| {
                selected_tree_context.as_ref().is_some_and(|selected| {
                    objects::context_for_selection_kind(
                        &model,
                        &selected.context,
                        kind,
                        retained.as_ref(),
                    )
                    .is_some()
                })
            };
            let properties_available = selected_tree_context.as_ref().is_some_and(|selected| {
                matches!(
                    &selected.context,
                    objects::TreeContext::Matrix { .. }
                        | objects::TreeContext::Row { .. }
                        | objects::TreeContext::Column { .. }
                        | objects::TreeContext::Key { .. }
                        | objects::TreeContext::Component {
                            part_id: Some(_),
                            ..
                        }
                )
            }) && (show_position_inspector
                || pcb_matrix_transform_inspector.projection.is_some());
            let on_show_properties = {
                let runtime = runtime.clone();
                let adapter = adapter.clone();
                let owner = layout_owner.clone();
                let mut objects_open = objects_open;
                let mut inspect_open = inspect_open;
                move |_| {
                    if !pcb_owner_is_current(&runtime, workspace, &adapter, &owner)
                        || !properties_available
                    {
                        return;
                    }
                    pin_inspector_on_desktop(inspector_panel_settings);
                    objects_open.set(false);
                    inspect_open.set(true);
                }
            };
            workspace_callbacks
                .pcb_transform_properties
                .replace(Box::new(on_show_properties));
            let transform = objects::LayoutTransformMenuMount {
                properties_available,
                column_available: supports_kind(objects::LayoutSelectionKind::Column),
                row_available: supports_kind(objects::LayoutSelectionKind::Row),
                on_selection_kind: workspace_callbacks.pcb_selection_kind,
                on_show_properties: workspace_callbacks.pcb_transform_properties,
            };
            workspace_composition::WorkspaceToolbarInput::Pcb(Box::new(
                pcb_workspace::ToolbarInput {
                    command_label: "PCB commands".to_owned(),
                    menu_owner_key: format!("{layout_owner:?}"),
                    selection_kind: layout_selection_kind(),
                    snap_settings: layout_snap_settings.read().clone(),
                    transform,
                    align: pcb_align.clone(),
                    on_selection_kind: workspace_callbacks.pcb_selection_kind,
                    on_snap_intent: workspace_callbacks.pcb_snap_intent,
                },
            ))
        }
        "Keymap" => {
            let on_view_mode = design_consumer_view_mode_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                layout_owner.clone(),
                layout_assembly_3d,
            );
            workspace_composition::WorkspaceToolbarInput::Keymap(
                shared_viewer::DesignViewToolbarProps {
                    label: "Keymap".into(),
                    detail: "Layers & key behaviors".into(),
                    assembly_3d: layout_assembly_3d(),
                    footprints_visible: (layer_visibility.footprints)()
                        && !(layer_visibility.hidden)().contains("Footprints"),
                    on_view_mode,
                    on_toggle_footprints: workspace_callbacks.toggle_footprints,
                },
            )
        }
        "Keycaps" => {
            let on_view_mode = design_consumer_view_mode_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                layout_owner.clone(),
                layout_assembly_3d,
            );
            workspace_composition::WorkspaceToolbarInput::Keycaps(
                shared_viewer::DesignViewToolbarProps {
                    label: "Keycaps".into(),
                    detail: "Profiles, legends & fit".into(),
                    assembly_3d: layout_assembly_3d(),
                    footprints_visible: (layer_visibility.footprints)()
                        && !(layer_visibility.hidden)().contains("Footprints"),
                    on_view_mode,
                    on_toggle_footprints: workspace_callbacks.toggle_footprints,
                },
            )
        }
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
        "PCB" => Some(workspace_composition::WorkspaceCanvasInput::Pcb(Box::new(
            pcb_workspace::CanvasInput {
                snapshot: snapshot.clone(),
                scope: render_scope.clone(),
                view_box: view_box.clone(),
                selected_ids: model.selected_part_ids.clone(),
                generation: render_generation,
                handlers: canvas_handlers,
                on_empty_hit: workspace_callbacks.pcb_empty_hit,
                on_part_hit: workspace_callbacks.pcb_part_hit,
                on_part_pointer_down: workspace_callbacks.pcb_part_pointer_down,
            },
        ))),
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
                generation_ready: mechanical_settings.generation_ready,
                instance_scope_pending,
                mechanical_settings: mechanical_settings.props.clone(),
            },
        ))),
        "Parts" => Some(workspace_composition::WorkspaceCanvasInput::Parts(
            parts_workspace::CanvasInput {
                controller_back: part_placement.controller_back,
                snapshot: snapshot.clone(),
                scope: current_scope.clone(),
                query: parts_query,
                selected: parts_selection,
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
    let on_component_inspector_action = EventHandler::new({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let lifetime = layout_component_inspector_lifetime.clone();
        move |action: inspector::LayoutComponentInspectorAction| {
            dispatch_layout_component_inspector_action(
                &runtime,
                &adapter,
                &owner,
                &lifetime,
                workspace,
                inspect_open,
                action,
            )
        }
    });
    let inspector_input = match active_workspace {
        "Keymap" => workspace_composition::WorkspaceInspectorInput::Keymap(Box::new(
            keymap_workspace::InspectorInput {
                view: keymap_view.clone(),
                scope: render_scope.clone(),
                layer_actions,
                active_layer_id: keymap_layer_id(),
                selected_key_id: model.selected_part_ids.first().cloned(),
                on_layer: workspace_callbacks.keymap_layer,
                on_export: workspace_callbacks.keymap_export,
                firmware_export_enabled: keymap_view
                    .as_ref()
                    .is_some_and(|view| !view.keys.is_empty())
                    || binding_actions
                        .encoder_projection
                        .as_ref()
                        .is_some_and(|projection| !projection.rows.is_empty()),
                on_select_key: workspace_callbacks.keymap_select,
                binding_actions,
                macro_actions,
                admission_token: snapshot.token,
                admission_revision: snapshot.document.revision,
                scope_generation: (adapter.generation)(),
            },
        )),
        "Case" => workspace_composition::WorkspaceInspectorInput::Case(Box::new(
            case_workspace::InspectorInput {
                physical_setup: physical_setup_mount.clone(),
                mechanical_settings: mechanical_settings.clone(),
                instance_scope_pending,
                scope: current_scope.clone(),
                scene: case_scene,
                case_selection,
                selected_layer_id: case_selected_layer_id,
                selected_context: case_selected_context,
                on_show_configured_board: workspace_callbacks.show_configured_board,
                on_display: workspace_callbacks.case_display,
            },
        )),
        "Parts" => workspace_composition::WorkspaceInspectorInput::Parts(Box::new(
            parts_workspace::InspectorInput {
                snapshot: snapshot.clone(),
                scope: current_scope.clone(),
                query: parts_query,
                selected: parts_selection,
                selected_context: adapter.selected_context,
                on_place_controller: part_placement.on_place_controller,
                on_place_component: part_placement.on_place_component,
                controller_placement_enabled: part_placement.controller_placement_enabled,
                placement_busy: part_placement.busy,
                placement_error: part_placement.error.clone(),
                layout_target,
            },
        )),
        "PCB" => {
            workspace_composition::WorkspaceInspectorInput::Pcb(pcb_wiring_source.map(|source| {
                let part_position = pcb_wiring::part_position_projection(
                    &model,
                    &render_scope,
                    selected_tree_context.as_ref(),
                    render_generation,
                );
                let firmware_position_projection = pcb_wiring::firmware_position_projection(
                    &source,
                    render_generation,
                    &pcb_wiring_mount.resolution,
                );
                let firmware_feedback =
                    firmware_position_actions
                        .feedback
                        .clone()
                        .filter(|feedback| {
                            feedback.target.is_visible(
                                &source.ui_scope,
                                render_generation,
                                &firmware_position_projection,
                            )
                        });
                let firmware_controls = rsx! {
                    firmware_positions::FirmwareKeymapPanel {
                        projection: firmware_position_projection.clone(),
                        feedback: firmware_feedback.clone(),
                        editable: firmware_position_actions.editable,
                        on_change: firmware_position_actions.on_edit,
                    }
                };
                Box::new(pcb_wiring::PcbWiringInspectorProps {
                    source,
                    resolution: pcb_wiring_mount.resolution.clone(),
                    firmware_positions: firmware_position_projection,
                    firmware_feedback,
                    firmware_controls,
                    part_net_actions: pcb_part_net_actions.clone(),
                    part_input_actions: part_input_actions.clone(),
                    on_firmware_edit: firmware_position_actions.on_edit,
                    on_resolve: pcb_wiring_mount.on_resolve,
                    on_choose_controller: part_placement.on_choose_controller,
                    on_edit_board_wiring: workspace_callbacks.pcb_wiring_edit_board,
                    mode_actions: pcb_wiring_mode_actions.clone(),
                    pin_actions: pcb_wiring_pin_actions.clone(),
                    apply_actions: pcb_wiring_apply_actions.clone(),
                    protected_remap_actions: pcb_wiring_protected_remap_actions.clone(),
                    matrix_transform_inspector: pcb_matrix_transform_inspector.clone(),
                    part_position,
                    on_part_position: workspace_callbacks.pcb_part_position,
                })
            }))
        }
        "Keycaps" => {
            let selected_key_id = model.selected_part_ids.first().cloned();
            let mechanical_layer_ids = current_scope
                .as_ref()
                .zip(model.accepted.as_ref())
                .and_then(|(scope, snapshot)| current_case_layer_ids(&runtime, scope, snapshot))
                .unwrap_or_else(|| Rc::from([]));
            let settings_editor = keycaps_view.as_deref().and_then(|view| {
                let selected = keycaps_settings::project_selected_key(
                    &document,
                    view,
                    selected_key_id.as_deref(),
                )?;
                keycaps_settings_actions
                    .clone()
                    .map(|actions| (selected, actions))
            });
            workspace_composition::WorkspaceInspectorInput::Keycaps(Box::new(
                keycaps_workspace::InspectorInput {
                    view: keycaps_view.clone(),
                    document: Rc::new((*document).clone()),
                    selected_key_id,
                    on_select_key: workspace_callbacks.keycaps_select,
                    settings_editor,
                    settings_actions: keycaps_settings_actions.clone(),
                    fit_state: keycaps_fit_state.state.clone(),
                    mechanical_layer_ids,
                    fit_retry: keycaps_fit_state.on_retry,
                    fit_navigate: workspace_callbacks.keycaps_finding,
                    on_export: {
                        let runtime = runtime.clone();
                        EventHandler::new(move |_: ()| runtime.export_keycaps_step())
                    },
                },
            ))
        }
        _ => workspace_composition::WorkspaceInspectorInput::Layout(Box::new(
            layout_workspace::InspectorInput {
                context_title: context_summary
                    .as_ref()
                    .map(|summary| summary.title.clone()),
                context_detail: context_summary
                    .as_ref()
                    .and_then(|summary| summary.detail.clone()),
                show_position_inspector,
                component_inspector: component_inspector.clone(),
                on_component_inspector_action,
                matrix_inspector,
                key_size,
                matrix_transform_inspector,
                outline_inspector: outline_inspector.clone().map(Box::new),
            },
        )),
    };
    let selected_bridge = selected_tree_context.as_ref().and_then(|selected| {
        let objects::TreeContext::Bridge {
            board_id,
            bridge_id,
        } = &selected.context
        else {
            return None;
        };
        snapshot
            .scene
            .board_outline_scenes
            .iter()
            .find(|scene| scene.board_id == *board_id)
            .and_then(|scene| scene.bridges.iter().find(|bridge| bridge.id == *bridge_id))
    });
    let active_part_position = model.selected_part_ids.first().and_then(|selected_id| {
        visible
            .iter()
            .find(|part| part.id == *selected_id)
            .map(|part| {
                scene
                    .transforms
                    .iter()
                    .find(|transform| transform.id == part.id)
                    .map(|transform| transform.pose.at)
                    .unwrap_or(part.pose.at)
            })
    });
    let controller_guide_hidden = part_placement.projection.is_some()
        || part_placement.busy
        || (active_workspace == "Parts"
            && parts_query().trim() == "controller"
            && guide_preferences()
                .as_ref()
                .is_some_and(|preferences| preferences.current_stage == SetupGuideStage::Wiring));
    let guide = guide_preferences().filter(|preferences| {
        preferences.project_id == document.id
            && preferences.open
            // Opening Matrix Setup from the guide temporarily reveals the normal Layout
            // Objects panel; the guide preference remains intact and returns on cancel.
            && !(active_workspace == "Layout"
                && (matrix_setup.projection.is_some()
                    || mirrored_pair.form.is_some()
                    || mirrored_pair.placement.is_some()))
            && !controller_guide_hidden
    });
    let outline_pitch = {
        let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &layout_owner);
        matrix_snap_parameters(&model, &render_scope, &adapter, retained.as_ref())
            .0
            .unwrap_or(Vec2 { x: 19.05, y: 19.05 })
    };
    let outline_snap_origins = visible
        .iter()
        .map(|part| {
            let definition = definitions.get(part.definition_id.as_str()).copied();
            let keycap = definition
                .filter(|definition| definition.kind == PartKind::Switch)
                .and_then(|definition| part.keycap.or(definition.keycap));
            let local = if let Some(size) = keycap {
                vec![
                    Vec2 {
                        x: -size.x / 2.0,
                        y: -size.y / 2.0,
                    },
                    Vec2 {
                        x: size.x / 2.0,
                        y: -size.y / 2.0,
                    },
                    Vec2 {
                        x: size.x / 2.0,
                        y: size.y / 2.0,
                    },
                    Vec2 {
                        x: -size.x / 2.0,
                        y: size.y / 2.0,
                    },
                ]
            } else {
                definition.map_or_else(Vec::new, |definition| definition.courtyard.clone())
            };
            let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
            let world = local
                .into_iter()
                .map(|point| Vec2 {
                    x: part.pose.at.x + point.x * cos - point.y * sin,
                    y: part.pose.at.y + point.x * sin + point.y * cos,
                })
                .collect();
            crate::presentation::outline_snapping::Origin {
                reference: part.reference.clone(),
                center: part.pose.at,
                polygon: crate::presentation::outline_snapping::convex_hull(world),
            }
        })
        .collect::<Vec<_>>();
    let outline_snap_settings = layout_snap_settings.read().clone();
    let outline_overlay_key = outline_inspector
        .as_ref()
        .map(outline_lifecycle::OutlineInspectorProjection::canvas_edit_key);
    let name_value = guide_name_draft()
        .filter(|(project_id, _)| project_id == &document.id)
        .map(|(_, name)| name)
        .unwrap_or_else(|| document.name.clone());
    let name_project_id = document.id.clone();
    let on_name_change = move |name: String| {
        guide_name_draft.set(Some((name_project_id.clone(), name)));
    };
    let name_runtime = runtime.clone();
    let mut name_draft = guide_name_draft;
    let expected_document = document.clone();
    let expected_token = snapshot.token;
    let on_name_commit = move |_| {
        let current = name_runtime.model();
        let Some(current_snapshot) = current.accepted.as_ref() else {
            return;
        };
        if current_snapshot.token != expected_token
            || current_snapshot.document.id != expected_document.id
            || current_snapshot.document.revision != expected_document.revision
        {
            name_draft.set(Some((
                current_snapshot.document.id.clone(),
                current_snapshot.document.name.clone(),
            )));
            return;
        }
        let proposed = name_draft()
            .filter(|(project_id, _)| project_id == &expected_document.id)
            .map(|(_, value)| value.trim().to_owned())
            .unwrap_or_default();
        if proposed.is_empty() || proposed == expected_document.name {
            name_draft.set(Some((
                expected_document.id.clone(),
                expected_document.name.clone(),
            )));
            return;
        }
        let mut replacement = expected_document.as_ref().clone();
        replacement.name = proposed;
        let operation_id = name_runtime.operation();
        name_runtime.submit(Event::Edit {
            operation_id,
            command: EditCommand {
                base_revision: expected_document.revision,
                transaction_id: format!("project-name-{}", operation_id.0),
                phase: EditPhase::Commit,
                target_ids: vec![expected_document.id.clone()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
            },
        });
    };
    let guide_runtime = runtime.clone();
    let mut guide_adapter = adapter.clone();
    let mut guide_preferences_for_stage = guide_preferences;
    let guide_workspace_for_stage = workspace;
    let guide_project_id = document.id.clone();
    let on_stage_change = move |stage: SetupGuideStage| {
        let Some(mut preferences) = guide_preferences_for_stage()
            .filter(|preferences| preferences.project_id == guide_project_id)
        else {
            return;
        };
        preferences.current_stage = stage;
        preferences.open = true;
        guide_preferences_for_stage.set(Some(preferences));
        setup_guide::activate_stage(
            stage,
            guide_workspace_for_stage,
            requested_workspace_panel,
            objects_open,
            inspect_open,
            objects_panel_settings,
            inspector_panel_settings,
        );
        guide_adapter.selected_context.set(None);
        guide_adapter.anchor_scope.set(None);
        guide_runtime.submit(Event::SelectParts {
            operation_id: guide_runtime.operation(),
            part_ids: Vec::new(),
            range_part_ids: Vec::new(),
            mode: SelectionMode::Replace,
        });
    };
    let mut guide_workspace_for_action = workspace;
    let mut guide_adapter_for_action = adapter.clone();
    let guide_runtime_for_action = runtime.clone();
    let settings_project_id = document.id.clone();
    let on_open_workspace = move |target: &'static str| {
        guide_workspace_for_action.set(target);
        setup_guide::reveal_panels(
            crate::setup_guide_state::GuideReveal::Settings,
            objects_open,
            inspect_open,
            objects_panel_settings,
            inspector_panel_settings,
        );
        let focus_runtime = guide_runtime_for_action.clone();
        let focus_project_id = settings_project_id.clone();
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(0).await;
            if *guide_workspace_for_action.peek() == target
                && focus_runtime
                    .model()
                    .accepted
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.document.id == focus_project_id)
            {
                setup_guide::focus_settings(target);
            }
        });
        guide_adapter_for_action.selected_context.set(None);
        guide_adapter_for_action.anchor_scope.set(None);
        guide_runtime_for_action.submit(Event::SelectParts {
            operation_id: guide_runtime_for_action.operation(),
            part_ids: Vec::new(),
            range_part_ids: Vec::new(),
            mode: SelectionMode::Replace,
        });
    };
    let mut guide_preferences_for_dismiss = guide_preferences;
    let guide_project_id_for_dismiss = document.id.clone();
    let on_dismiss_guide = move |_| {
        if let Some(mut preferences) = guide_preferences_for_dismiss()
            .filter(|preferences| preferences.project_id == guide_project_id_for_dismiss)
        {
            preferences.open = false;
            guide_preferences_for_dismiss.set(Some(preferences));
        }
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
                    if let Some(preferences) = guide {
                        setup_guide::ProjectSetupGuide {
                            stage: preferences.current_stage,
                            stage_detail: setup_guide::stage_detail(
                                preferences.current_stage,
                                &document,
                                &model.active_board_id,
                            ),
                            project_name: name_value,
                            on_name_change,
                            on_name_commit,
                            on_stage_change,
                            on_open_workspace,
                            on_open_matrix_setup: Some(matrix_setup.on_open),
                            on_choose_controller: Some(part_placement.on_choose_controller),
                            on_dismiss: on_dismiss_guide,
                            project_controls: if preferences.current_stage
                                == SetupGuideStage::Project
                            {
                                Some(pcb_physical_setup::controller::project_setup_controls(
                                    physical_setup_mount.clone(),
                                ))
                            } else {
                                None
                            },
                        }
                    } else {
                        {workspace_composition::objects(objects_input)}
                    }
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
                    if active_workspace == "Layout"
                        && let Some(projection) = mirrored_pair.form.clone()
                    {
                        objects::MirroredPairCanvasOverlay {
                            projection,
                            on_cancel: mirrored_pair.on_cancel,
                            on_preview: mirrored_pair.on_preview,
                        }
                    }
                    if matches!(active_workspace, "Layout" | "Keymap" | "Keycaps")
                        && layout_assembly_3d()
                    {
                        layout_viewer::LayoutCanonicalViewer {}
                    } else if active_workspace == "Layout" {
                        svg { class: "m1-canvas", view_box: "{view_box}", preserve_aspect_ratio: "xMidYMid meet", tabindex: "0", role: "group", "aria-label": "Keyboard layout; drag components, hold Shift for range selection, hold Space and drag to pan, or use position controls", onmounted: mount,
                    onpointerdown: start_pan, onpointermove: move_pointer, onpointerup: end_pointer, onpointercancel: cancel_pointer.clone(), onlostpointercapture: cancel_pointer, onkeydown: keyboard, onkeyup: key_up, onwheel: wheel,
                    g { transform: "scale(1,-1)",
                        if !(layer_visibility.hidden)().contains("Board") {
                            for contour in scene.board_contours.iter().filter(|b| b.board_id == model.active_board_id).flat_map(|b| &b.contours) {
                                polygon { points: polygon_points(&contour.points), class: if contour.hole { "m1-outline is-hole" } else { "m1-outline" } }
                            }
                        }
                        if let Some(projection) = outline_inspector.clone().filter(|projection| (projection.editing_points)()) {
                            outline_lifecycle::OutlinePointCanvasOverlay {
                                key: "{outline_overlay_key.as_deref().unwrap_or_default()}",
                                projection,
                                runtime: outline_lifecycle::OutlineRuntimeHandle::new(
                                    runtime.clone(),
                                ),
                                arbiter: canvas_interaction.clone(),
                                svg: svg.clone(),
                                view_x,
                                view_y,
                                width,
                                height,
                                snap_settings: outline_snap_settings.clone(),
                                pitch: outline_pitch,
                                origins: outline_snap_origins.clone(),
                            }
                        }
                        if let Some(bridge) = selected_bridge {
                            polygon { points: polygon_points(&bridge.points), class: "m1-outline-bridge-selected", "data-outline-bridge": bridge.id.clone() }
                        }
                        if !(layer_visibility.hidden)().contains("Keys") {
                            if let Some(active_pair) = mirrored_pair.placement.as_ref() {
                                {
                                    let pair = &active_pair.pair;
                                    let left_cells = crate::mirrored_pair_geometry::preview_cells(&active_pair.left_scene, &pair.matrix);
                                    let right_cells = crate::mirrored_pair_geometry::preview_cells(&active_pair.right_scene, &pair.right_preview);
                                    let line_half_height = f64::from(pair.matrix.rows) * pair.matrix.pitch.y / 2.0 + 16.0;
                                    rsx! {
                                        g { class: "m1-mirrored-pair-preview", "aria-label": "Mirrored pair placement preview", transform: "translate({pair.center.x} {pair.center.y})",
                                            line { class: "m1-mirror-pair-axis", x1: "0", x2: "0", y1: "{-line_half_height}", y2: "{line_half_height}" }
                                            for cell in left_cells {
                                                g { key: "left-{cell.row}-{cell.column}", transform: "translate({cell.center.x} {cell.center.y}) rotate({cell.rotation})",
                                                    rect { class: "m1-mirrored-pair-cell", x: "{-cell.size.x / 2.0}", y: "{-cell.size.y / 2.0}", width: "{cell.size.x}", height: "{cell.size.y}", rx: "1" }
                                                }
                                            }
                                            for cell in right_cells {
                                                g { key: "right-{cell.row}-{cell.column}", transform: "translate({cell.center.x} {cell.center.y}) rotate({cell.rotation})",
                                                    rect { class: "m1-mirrored-pair-cell", x: "{-cell.size.x / 2.0}", y: "{-cell.size.y / 2.0}", width: "{cell.size.x}", height: "{cell.size.y}", rx: "1" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
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
                                            let selection_kind = layout_selection_kind;
                                            let owner = layout_owner.clone();
                                            let tree_cell_anchor = tree_cell_anchor.clone();
                                            let canvas_interaction = canvas_interaction.clone();
                                            let mirrored_pair = mirrored_pair.clone();
                                            rsx! { rect { class: if selected { "m1-matrix-key is-selected" } else { "m1-matrix-key" }, x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}", rx: "0.9", transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation})", "data-matrix-id": "{matrix.id}", "data-row": "{cell.row}", "data-column": "{cell.column}",
                                                onpointerdown: move |event: PointerEvent| {
                                                    let Some(pointer) = event.data().try_as_web_event() else { return; };
                                                    if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair) && mirrored_pair.placement.is_some() { return; }
                                                    if canvas_interaction.current().is_some() { pointer.prevent_default(); pointer.stop_propagation(); return; }
                                                    if pointer.button() != 0 { return; }
                                                    if space_down.get() { return; }
                                                    pointer.prevent_default();
                                                    pointer.stop_propagation();
                                                    let current = runtime.model();
                                                    if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) { return; }
                                                    let Some(hit_context) = objects::context_for_cell(&current, &matrix_id, cell_row, cell_column) else { return; };
                                                    update_tree_cell_anchor(&tree_cell_anchor, &owner, &current, Some(&hit_context));
                                                    let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
                                                    let context = objects::context_for_selection_kind(&current, &hit_context, selection_kind(), retained.as_ref())
                                                        .map(|projection| projection.context)
                                                        .unwrap_or(hit_context);
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
                                let canvas_interaction = canvas_interaction.clone();
                                let adapter = adapter.clone(); let render_scope_for_hit = render_scope.clone();
                                let mut selected_context = adapter.selected_context;
                                let generation_for_hit = render_generation;
                                let range_ids = visible_ids.clone();
                                let selection_kind = layout_selection_kind;
                                let owner = layout_owner.clone();
                                let tree_cell_anchor = tree_cell_anchor.clone();
                                let mirrored_pair = mirrored_pair.clone();
                                rsx! { if layer_visible { g { key: "{part.id}", class: "m1-scene-part", transform: "translate({pose.at.x},{pose.at.y}) rotate({pose.rotation}) {side_transform}", "data-part-id": "{part.id}",
                                    onpointerdown: move |event: PointerEvent| {
                                        let Some(pointer) = event.data().try_as_web_event() else { return; };
                                        if pointer.button() != 0 { return; }
                                        if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair) && mirrored_pair.placement.is_some() { return; }
                                        if canvas_interaction.current().is_some() { pointer.prevent_default(); pointer.stop_propagation(); return; }
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
                                        if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) { return; }
                                        let Some(hit_context) = objects::context_for_part(&current, &id) else { return; };
                                        update_tree_cell_anchor(&tree_cell_anchor, &owner, &current, Some(&hit_context));
                                        let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
                                        let projection = objects::context_for_selection_kind(&current, &hit_context, selection_kind(), retained.as_ref());
                                        let target_ids = projection.as_ref().map(|projection| projection.part_ids.clone())
                                            .unwrap_or_else(|| selection::resolve_context(&current, &hit_context).unwrap_or_default());
                                        let context = projection.map(|projection| projection.context).unwrap_or(hit_context);
                                        if !current.selected_part_ids.contains(&id)
                                            || mode != SelectionMode::Replace
                                            || target_ids != current.selected_part_ids
                                        {
                                            selection::submit_canvas_selection(&runtime, &adapter, &render_scope_for_hit, generation_for_hit, context.clone(), mode, if mode == SelectionMode::Range { range_ids.as_ref().clone() } else { Vec::new() });
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
                        if let Some(placement) = part_placement.projection.as_ref() {
                            { let definition = placement.pending.definition.clone();
                              let at = placement.pending.part.pose.at;
                              let courtyard = polygon_points(&definition.courtyard);
                              let reference = placement.pending.part.reference.clone();
                              rsx! {
                                g {
                                    class: "m1-part-placement-preview",
                                    transform: "translate({at.x},{at.y})",
                                    "aria-hidden": "true",
                                    polygon { points: "{courtyard}", class: "m1-part-placement-envelope" }
                                    FootprintGraphics {
                                        definition: definition.clone(),
                                        parameters: placement.pending.part.generator_parameters.clone(),
                                    }
                                    for pad in &definition.pads {
                                        g { transform: "translate({pad.at.x} {pad.at.y}) rotate({pad.rotation.unwrap_or(0.0)})",
                                            if pad.plated != Some(false) {
                                                rect { class: "m1-part-pad", x: "{-pad.size.x / 2.0}", y: "{-pad.size.y / 2.0}", width: "{pad.size.x}", height: "{pad.size.y}" }
                                            }
                                            if let Some(drill) = pad.drill { circle { class: "m1-part-drill", r: "{drill / 2.0}" } }
                                        }
                                    }
                                    text { transform: "scale(1,-1)", text_anchor: "middle", class: "m1-part-label", x: "0", y: "-5.2", "{reference}" }
                                }
                              }
                            }
                        }
                        keycaps_finding_marker::FocusedFindingMarker {
                            workspace: active_workspace.to_owned(),
                            scope: Some(render_scope.clone()),
                            token: Some(snapshot.token),
                            revision: Some(snapshot.document.revision),
                            active_board_id: model.active_board_id.clone(),
                            finding: focused_keycaps_finding(),
                            markers: Rc::from(snapshot.scene.finding_markers.clone()),
                        }
                    }
                        }
                        CanvasLayers {
                            trigger_id: String::from("m1-layers-trigger"),
                            list_id: String::from("m1-layers-list"),
                            groups: canvas_layers::layout_groups(),
                        }
                    } else if active_workspace == "Export" {
                        ExportPanel { zmk_firmware: Some(zmk_firmware_export_panel) }
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
                if active_workspace == "Layout" {
                    objects::LayoutSelectionSnapStatus {
                        snap_settings: layout_snap_settings.read().clone(),
                        active_part_position,
                    }
                    if let Some(guide) = model.snap_guide.as_ref() {
                        span { class: "m1-layout-snap-guide", role: "status", "{guide.label}" }
                    }
                }
                if active_workspace == "Keymap" && !layout_assembly_3d() {
                    keymap::KeymapViewControls {
                        board_available: keymap_canvas_bounds.is_some(),
                        selection_available: keymap_selection_available,
                        on_fit_board: on_fit_keymap_board,
                        on_fit_selection: on_fit_keymap_selection,
                    }
                }
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
    coordinates_at(
        svg,
        pointer.client_x(),
        pointer.client_y(),
        x,
        y,
        width,
        height,
    )
}

fn coordinates_at(
    svg: &Rc<RefCell<Option<SvgElement>>>,
    client_x: i32,
    client_y: i32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Option<Vec2> {
    let surface = svg.borrow();
    let rect = surface.as_ref()?.get_bounding_client_rect();
    pointer_location(&rect, client_x, client_y, x, y, width, height).map(|location| location.world)
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
