//! Presentation drafts and DOM input are separate from the durable session state.
pub(crate) use boardstudio_web_layout::board_inspector;
pub(crate) use boardstudio_web_ui_shared::canvas_layers;
mod canvas_status_footer;
pub(crate) use boardstudio_web_case::case_controller;
pub(crate) use boardstudio_web_case::case_display;
pub(crate) use boardstudio_web_case::case_viewer;
mod case_workspace;
mod context_summary;
mod empty_board_canvas;
mod export_workspace;
pub(crate) use boardstudio_web_pcb::firmware_positions;
pub(crate) use boardstudio_web_ui_shared::geometry_scripts;
mod inspector;
pub(crate) use boardstudio_web_keycaps::keycaps_finding_marker;
pub(crate) use boardstudio_web_keycaps::keycaps_fit;
pub(crate) use boardstudio_web_keycaps::keycaps_navigation;
pub(crate) use boardstudio_web_keycaps::keycaps_scene;
pub(crate) use boardstudio_web_keycaps::keycaps_settings;
mod keycaps_workspace;
pub(crate) use boardstudio_web_keymap::keymap;
mod keymap_workspace;
pub(crate) use boardstudio_web_ui_shared::layout_camera;
mod layout_component_edits;
#[cfg(test)]
mod layout_component_inspector_tests;
#[cfg(test)]
mod layout_remainder_tests;
pub(crate) use boardstudio_web_layout::layout_findings;
pub(crate) use boardstudio_web_layout::layout_viewer;
// Shared UI vocabulary lives in `boardstudio-web-ui-model`; re-export it here so
// presentation modules keep addressing it as `super::selection`, `super::InstanceSelection`
// and so on.
#[allow(unused_imports)]
#[cfg(test)]
pub(crate) use boardstudio_web_case::test_contexts::{
    use_case_generation_readiness_test_bridge, use_case_viewer_test_contexts,
};
pub(crate) use boardstudio_web_ui_model::selection::{
    active_board_scope_matches, current_layout_owner, layout_owner_is_current,
};
#[cfg(test)]
pub(crate) use boardstudio_web_ui_model::state::use_empty_test_instance_selection;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::{
    CaseGenerationState, CompactPanelState, Drag, InstanceSelection, LayerVisibility,
    LayoutOwnerIdentity, ProjectMenuPage, ResolvedTheme, WorkspaceState,
};
pub(crate) use boardstudio_web_ui_model::state::{PreferenceStorageWarning, ThemeState};
pub(crate) use boardstudio_web_ui_model::svg_coordinates::{
    PointerLocation, coordinates, coordinates_at, pointer_location,
};
pub(crate) use boardstudio_web_ui_model::{canvas_interaction, instance_selection, selection};
pub(crate) use boardstudio_web_ui_shared::panels::browse_parts_workspace;
pub(crate) use boardstudio_web_ui_shared::project_menu::close_project_menu;
mod layout_workspace;
pub(crate) use boardstudio_web_case::mechanical_settings;
pub(crate) use boardstudio_web_case::mechanical_settings_mount;
pub(crate) use boardstudio_web_layout::objects;
pub(crate) use boardstudio_web_layout::outline_lifecycle;
pub(crate) use boardstudio_web_layout::outline_snapping;
pub(crate) use boardstudio_web_layout::part_placement;
pub(crate) use boardstudio_web_library::library;
pub(crate) use boardstudio_web_parts::parts;
pub(crate) use boardstudio_web_ui_shared::panels;
use part_placement::{LayoutPlacementCancellation, layout_view_mode_handler};
mod parts_workspace;
pub(crate) use boardstudio_web_pcb::pcb_board_reference;
pub(crate) use boardstudio_web_pcb::pcb_layers;
pub(crate) use boardstudio_web_pcb::pcb_module_footprints;
pub(crate) use boardstudio_web_pcb::pcb_module_inspector;
pub(crate) use boardstudio_web_pcb::pcb_physical_setup;
pub(crate) use boardstudio_web_pcb::pcb_scene;
pub(crate) use boardstudio_web_pcb::pcb_wiring;
mod pcb_workspace;
pub(crate) use boardstudio_web_case::shared_viewer;
pub(crate) use boardstudio_web_layout::setup_guide;
mod workbench_shortcuts;
mod workspace_composition;
mod zmk_firmware_export;

use crate::case_generation_lifecycle::AutomaticCaseGeneration;
pub(crate) use boardstudio_web_ui_shared::footprint_graphics;
use canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner};
use canvas_layers::CanvasLayers;
use library::Library;
pub(crate) use mechanical_settings::{MechanicalSettings, MechanicalSettingsProps};
pub(crate) use mechanical_settings_mount::MechanicalSettingsMount;
use panels::{
    InspectorPanel, ObjectsPanel, PanelMode, PanelSettings, PanelSide, use_panel_settings,
};
use parts::{PartsQuery, PartsSelection};
use selection::{ReentrancyReset, SelectionAdapter};
use setup_guide::{PendingNewKeyboard, SetupGuidePreferences, SetupGuideRequest, SetupGuideStage};
use zmk_firmware_export::use_export_panel_input;
#[cfg(test)]
use zmk_firmware_export::{ZmkFirmwareExportPanelInput, ZmkFirmwareExportRow};

use crate::runtime::Runtime;
#[cfg(test)]
use boardstudio_application::SnapshotToken;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, ReadModel, Scope, SelectionMode,
    TerminalOutcome,
};
use boardstudio_core::model::{
    Contour, EditCommand, EditOperation, EditPhase, Matrix, MatrixSplayAffect, Part,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct LayoutFindingReturnTarget {
    owner: LayoutOwnerIdentity,
    selection: objects::ScopedTreeContext,
    destination: objects::TreeContext,
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
            scope_transition: None,
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
pub(super) struct ExportReturnWorkspace(pub(super) Signal<&'static str>);

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
    mounted_module_select: EventHandler<String>,
    pcb_wiring_edit_board: EventHandler<()>,
    layout_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pcb_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pcb_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pcb_transform_properties: EventHandler<()>,
    case_action: EventHandler<case_workspace::TreeAction>,
    case_display: EventHandler<case_workspace::DisplayRequest>,
    keymap_layer: EventHandler<String>,
    keymap_export: EventHandler<()>,
    show_configured_board: EventHandler<String>,
    open_geometry_scripts: EventHandler<()>,
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
    let selection_retention = use_hook(|| {
        Rc::new(RefCell::new(
            crate::matrix_transform_lifecycle::SelectionRetention::default(),
        ))
    });
    let selection_membership = use_hook(|| {
        Rc::new(RefCell::new(
            crate::matrix_transform_lifecycle::SelectionMembershipCache::default(),
        ))
    });
    let observed_scope = use_hook({
        let runtime = runtime.clone();
        move || Rc::new(RefCell::new(runtime.scope()))
    });
    let observed_token = use_hook({
        let runtime = runtime.clone();
        move || {
            Rc::new(RefCell::new(
                runtime.model().accepted.as_ref().map(|s| s.token),
            ))
        }
    });
    let reconciling_scope = use_hook(|| Rc::new(Cell::new(false)));
    use_hook({
        let runtime = runtime.clone();
        let active = active.clone();
        let weak_runtime = Rc::downgrade(&runtime);
        let adapter = adapter.clone();
        let selection_retention = selection_retention.clone();
        let selection_membership = selection_membership.clone();
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
                let next_token = runtime
                    .model()
                    .accepted
                    .as_ref()
                    .map(|snapshot| snapshot.token);
                let previous_token = {
                    let mut observed = observed_token.borrow_mut();
                    std::mem::replace(&mut *observed, next_token)
                };
                let previous_scope = {
                    let mut observed = observed_scope.borrow_mut();
                    std::mem::replace(&mut *observed, next_scope.clone())
                };
                if previous_scope != next_scope {
                    selected_context.set(None);
                    anchor_scope.set(None);
                    let previous_generation = *generation.peek();
                    generation += 1;
                    adapter.record_scope_transition(selection::ScopeTransition {
                        previous_scope,
                        previous_token,
                        next_scope: next_scope.clone(),
                        next_token,
                        previous_generation,
                        next_generation: *generation.peek(),
                    });
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

                let membership = selection_membership.borrow_mut().project(&model, |model| {
                    crate::matrix_transform_lifecycle::SelectionMembership {
                        eligible: selection::eligible_live_ids(model),
                        live: selection::live_board_ids(model),
                    }
                });
                let selected_context = (adapter.selected_context)().filter(|selected| {
                    matches!(&selected.context, objects::TreeContext::Key { .. })
                });
                let selected_ids = selection_retention.borrow_mut().reconcile(
                    selected_context.as_ref(),
                    &model.selected_part_ids,
                    &membership.eligible,
                    &membership.live,
                );
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
                    let membership = selection_membership
                        .borrow_mut()
                        .project(&current, |model| {
                            crate::matrix_transform_lifecycle::SelectionMembership {
                                eligible: selection::eligible_live_ids(model),
                                live: selection::live_board_ids(model),
                            }
                        });
                    if (adapter.anchor_scope)().as_ref() == Some(scope)
                        && current
                            .selection_anchor_id
                            .as_ref()
                            .is_none_or(|anchor| !membership.eligible.iter().any(|id| id == anchor))
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
    let mut objects_open = use_signal(|| false);
    let mut inspector_open = use_signal(|| false);
    use_context_provider(|| CompactPanelState {
        objects_open,
        inspector_open,
    });
    let mut return_workspace = use_signal(|| "Layout");
    use_context_provider(|| ExportReturnWorkspace(return_workspace));
    use_effect(move || {
        let active = workspace();
        if active != "Export" {
            return_workspace.set(active);
        } else {
            objects_open.set(false);
            inspector_open.set(false);
        }
    });
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
    let preference_warning = use_signal(|| false);
    use_context_provider(|| PreferenceStorageWarning(preference_warning));
    let theme = use_signal(move || panels::read_theme_preference(preference_warning));
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
    let save_state = if runtime.model().accepted.is_none() {
        "ready"
    } else {
        durability_state(&runtime.model().durability)
    };
    let mut project_menu_page = use_signal(|| ProjectMenuPage::Project);
    rsx! {
        link { rel: "stylesheet", href: "assets/m1.css" }
        link { rel: "stylesheet", href: "assets/firmware-keymap-panel.css" }
        main { class: "m1-workbench",
            onkeydown: {
                let runtime = runtime.clone();
                move |event: KeyboardEvent| {
                    workbench_shortcuts::handle_history_shortcut(runtime.clone(), event);
                }
            },
            header { class: "m1-topbar",
                details { class: "m1-project-menu", onkeydown: move |event: KeyboardEvent| {
                    if event.data().key().to_string() == "Escape" {
                        event.prevent_default();
                        close_project_menu();
                    }
                },
                    summary { role: "button", "aria-label": "Project", "aria-controls": "m1-project-menu-dropdown", title: "Project menu — {project_name}", onclick: move |_| project_menu_page.set(ProjectMenuPage::Project),
                        svg { class: "m1-project-mark", view_box: "0 0 30 30", fill: "none", stroke: "currentColor", stroke_width: "1.5", "aria-hidden": "true", path { d: "M4 4h22v22H4z" }, path { d: "m8 20 5-10 4 8 3-5 3 7" }, circle { cx: "13", cy: "10", r: "1.3" } }
                        span { class: "m1-project-name", "{project_name}" }
                        svg { class: "m1-project-chevron", view_box: "0 0 20 20", fill: "none", stroke: "currentColor", stroke_width: "1.5", "aria-hidden": "true", path { d: "m5 7 5 5 5-5" } }
                    }
                    Library { project_menu: true, menu_page: Some(project_menu_page) }
                }
                details { class: "m1-save-state", "data-state": "{save_state}", onkeydown: move |event: KeyboardEvent| {
                    if event.data().key().to_string() == "Escape" {
                        event.prevent_default();
                        if let Some(menu) = event.data().try_as_web_event().and_then(|event| event.current_target()).and_then(|target| target.dyn_into::<web_sys::Element>().ok()) {
                            let _ = menu.remove_attribute("open");
                            if let Some(summary) = menu.query_selector("summary").ok().flatten().and_then(|element| element.dyn_into::<HtmlElement>().ok()) {
                                let _ = summary.focus();
                            }
                        }
                    }
                },
                    summary {
                        aria_label: match save_state { "saved" => "Saved locally", "saving" => "Saving locally", "failed" => "Local save failed", _ => "Local save status unavailable" },
                        i { "aria-hidden": "true" }
                    }
                    div { role: "status",
                        if save_state == "saved" { "Changes are saved in this browser. Use Save project copy for a portable backup." }
                        else if save_state == "saving" { "Saving changes in this browser…" }
                        else if save_state == "failed" { "Changes could not be saved locally. Check available browser storage before editing further." }
                        else { "Local save status is unavailable." }
                    }
                }
                WorkspaceNavigation {}
                if runtime.model().accepted.is_some() && workspace() != "Export" {
                    panels::CompactPanelControls {}
                }
                button { class: "m1-export-tab", id: "m1-tab-Export", "aria-pressed": "{workspace() == \"Export\"}", onclick: move |_| if workspace() == "Export" { workspace.set(return_workspace()) } else { workspace.set("Export") },
                    svg { view_box: "0 0 20 20", fill: "none", stroke: "currentColor", stroke_width: "1.5", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true", path { d: "M4 12v5h12v-5M10 13V3M6 7l4-4 4 4" } }
                    span { "Export" }
                }
            }
            if runtime.model().accepted.is_some() { Editor {} }
            else { LibraryLanding {} }
                RuntimeReportBanner {}
            if preference_warning() {
                p { role: "status", "aria-live": "polite", class: "m1-status",
                    "Preferences are session-only because browser storage is unavailable."
                }
            }
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
                for tab in tabs { option { value: "{tab}", selected: workspace() == tab, "{tab}" } }
                if workspace() == "Export" { option { value: "Export", selected: true, "Export" } }
            }
        }
    }
}

#[component]
fn TabIcon(name: &'static str) -> Element {
    rsx! { svg { class: "m1-tab-icon", view_box: "0 0 20 20", fill: "none", stroke: "currentColor", stroke_width: "1.5", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
        if name == "Layout" { path { d: "M3 14.5 14.5 3l2.5 2.5L5.5 17H3z" } path { d: "m11 6 3 3M3 17h14" } }
        else if name == "PCB" { rect { x: "3", y: "3", width: "14", height: "14", rx: "2" } circle { cx: "7", cy: "7", r: "1.2" } circle { cx: "13", cy: "13", r: "1.2" } path { d: "M8 7h3v3M7 8v3h3" } }
        else if name == "Keymap" { rect { x: "3", y: "4", width: "14", height: "12", rx: "2" } path { d: "M6 7h2M10 7h2M14 7h1M6 10h2M10 10h2M6 13h8" } }
        else if name == "Keycaps" { path { d: "m3 15 2-10h10l2 10zM5 5l2 4h6l2-4M7 9l-1 6m7-6 1 6" } }
        else if name == "Case" { path { d: "m10 2 7 4v8l-7 4-7-4V6zM3 6l7 4 7-4m-7 4v8" } }
        else { path { d: "M4 3h9l3 3v11H4z" } path { d: "M13 3v4h4M7 11h6M7 14h6" } }
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

#[cfg(test)]
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

fn apply_pending_splay_origin_pick(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
    pending: &mut Signal<Option<objects::MatrixTransformInspectorOwner>>,
    inspector: &objects::MatrixTransformInspectorMount,
    point: Vec2,
) -> bool {
    let Some(picked_owner) = pending.peek().clone() else {
        return false;
    };
    pending.set(None);
    let selected_is_current = adapter
        .selected_context
        .read()
        .as_ref()
        .is_some_and(|selected| {
            selected.scope == picked_owner.scope
                && selected.context == picked_owner.context
                && matches!(&selected.context, objects::TreeContext::Column { .. })
                && selection::context_is_current(
                    &runtime.model(),
                    &selected.scope,
                    &selected.context,
                )
        });
    let inspector_is_current = inspector
        .projection
        .as_ref()
        .is_some_and(|projection| projection.owner == picked_owner);
    if !layout_owner_is_current(runtime, workspace, adapter, owner)
        || !selected_is_current
        || !inspector_is_current
        || !inspector.pick_splay_origin(point)
    {
        runtime.report("Column selection changed. Pick the splay origin again.");
    }
    true
}

fn layout_finding_context(
    model: &ReadModel,
    target: &keycaps_fit::FindingNavigationTarget,
) -> Option<objects::TreeContext> {
    use keycaps_fit::FindingNavigationTarget as Target;
    match target {
        Target::Part { part_id, .. } => objects::component_context_for_finding_part(model, part_id),
        Target::Matrix { matrix_id, .. } => Some(objects::TreeContext::Matrix {
            matrix_id: matrix_id.clone(),
        }),
        Target::Outline { board_id } => Some(objects::TreeContext::Outline {
            board_id: board_id.clone(),
        }),
        Target::Board { board_id } => Some(objects::TreeContext::Outline {
            board_id: board_id.clone(),
        }),
        Target::Body { .. } | Target::MechanicalLayer { .. } => None,
    }
}

fn layout_finding_return_is_current(
    model: &ReadModel,
    owner: &LayoutOwnerIdentity,
    current_selection: Option<&objects::ScopedTreeContext>,
    target: &LayoutFindingReturnTarget,
) -> bool {
    if owner != &target.owner || owner.workspace != "Layout" {
        return false;
    }
    let Some(scope) = owner.scope.as_ref() else {
        return false;
    };
    let saved_selection_is_live =
        selection::context_is_current(model, scope, &target.selection.context)
            && match &target.selection.context {
                objects::TreeContext::Component {
                    part_id: Some(_), ..
                } => selection::resolve_context(model, &target.selection.context)
                    .is_some_and(|part_ids| !part_ids.is_empty()),
                _ => true,
            };
    target.selection.scope == *scope
        && saved_selection_is_live
        && current_selection.is_some_and(|selected| {
            selected.scope == *scope
                && selected.context == target.destination
                && selection::context_is_current(model, scope, &selected.context)
        })
}

fn focus_layout_finding_return_destination() -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return false;
    };
    for selector in [
        "#m1-inspector-panel-content .m1-layout-component-tabs [role='tab'][aria-selected='true']",
        "#m1-inspector-panel-content .m1-board-inspector :is(input, select, button):not(:disabled)",
        "#m1-inspector-panel-content .m1-outline-inspector :is(input, select, button):not(:disabled)",
    ] {
        if let Some(element) = document
            .query_selector(selector)
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlElement>().ok())
        {
            let _ = element.focus();
            return true;
        }
    }
    if let Some(element) = document
        .query_selector(
            "#m1-inspector-panel-content .m1-inspector-body, #m1-inspector-panel-content .m1-selected-context, #m1-inspector-panel-content .m1-board-inspector, #m1-inspector-panel-content .m1-outline-inspector",
        )
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
    {
        let _ = element.set_attribute("tabindex", "-1");
        let _ = element.focus();
        return true;
    }
    false
}

#[cfg(test)]
mod layout_finding_return_regression_tests {
    use super::*;
    use boardstudio_application::{AcceptedSnapshot, SessionEpoch};
    use boardstudio_core::model::{
        Board, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Readiness, SceneDelta, Side, Vec2,
    };
    use std::sync::Arc;
    use wasm_bindgen::{JsCast, closure::Closure};
    use wasm_bindgen_test::wasm_bindgen_test;

    fn fixture() -> (ReadModel, Scope, objects::ScopedTreeContext) {
        let scope = Scope {
            session_epoch: SessionEpoch(5),
            document_id: "finding-return-doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let mut document = ProjectDoc::empty("finding-return-doc", "Finding return fixture");
        document.revision = 9;
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["left-U1".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.definitions.push(PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: "controller".into(),
            name: "Controller".into(),
            kind: PartKind::Controller,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![],
            pads: vec![],
            models: None,
            generator: None,
            mechanical_profile: None,
        });
        document.parts.push(Part {
            keycap: None,
            outline: None,
            id: "left-U1".into(),
            definition_id: "controller".into(),
            reference: "U1".into(),
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        let model = ReadModel {
            accepted: Some(AcceptedSnapshot {
                token: SnapshotToken(13),
                session_epoch: scope.session_epoch,
                document: Arc::new(document),
                scene: Arc::new(SceneDelta {
                    module_scenes: vec![],
                    revision: 9,
                    transaction_id: "accepted-fixture".into(),
                    changed_ids: vec![],
                    transforms: vec![],
                    matrix_scenes: vec![],
                    contours: vec![],
                    board_contours: vec![],
                    board_readiness: vec![],
                    board_outline_scenes: vec![],
                    finding_markers: vec![],
                    findings: vec![],
                    readiness: Readiness {
                        layout: true,
                        outline: true,
                        pcb: true,
                        case_ready: false,
                    },
                }),
            }),
            active_board_id: scope.board_id.clone(),
            selected_part_ids: vec!["left-U1".into()],
            ..ReadModel::default()
        };
        let selection = objects::ScopedTreeContext {
            scope: scope.clone(),
            context: objects::TreeContext::Component {
                part_id: Some("left-U1".into()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            },
        };
        (model, scope, selection)
    }

    #[wasm_bindgen_test]
    fn outline_return_requires_the_same_live_owner_and_saved_selection() {
        let (model, scope, selection) = fixture();
        let owner = LayoutOwnerIdentity {
            scope: Some(scope.clone()),
            token: Some(SnapshotToken(13)),
            revision: Some(9),
            generation: 2,
            workspace: "Layout",
        };
        let destination = objects::TreeContext::Outline {
            board_id: scope.board_id.clone(),
        };
        let target = LayoutFindingReturnTarget {
            owner: owner.clone(),
            selection: selection.clone(),
            destination: destination.clone(),
        };
        let outline_selection = objects::ScopedTreeContext {
            scope,
            context: destination,
        };

        assert!(layout_finding_return_is_current(
            &model,
            &owner,
            Some(&outline_selection),
            &target,
        ));

        let empty_outline_selection = objects::ScopedTreeContext {
            scope: outline_selection.scope.clone(),
            context: outline_selection.context.clone(),
        };
        let empty_outline_target = LayoutFindingReturnTarget {
            selection: empty_outline_selection.clone(),
            ..target.clone()
        };
        assert!(layout_finding_return_is_current(
            &model,
            &owner,
            Some(&empty_outline_selection),
            &empty_outline_target,
        ));

        let component_target = LayoutFindingReturnTarget {
            destination: selection.context.clone(),
            ..target.clone()
        };
        assert!(layout_finding_return_is_current(
            &model,
            &owner,
            Some(&selection),
            &component_target,
        ));

        let mut changed_owner = owner.clone();
        changed_owner.revision = Some(10);
        assert!(!layout_finding_return_is_current(
            &model,
            &changed_owner,
            Some(&outline_selection),
            &target,
        ));

        let mut changed_scope_owner = owner.clone();
        changed_scope_owner
            .scope
            .as_mut()
            .expect("fixture scope")
            .board_id = "other-board".into();
        assert!(!layout_finding_return_is_current(
            &model,
            &changed_scope_owner,
            Some(&outline_selection),
            &target,
        ));

        let mut changed_model = model;
        Arc::make_mut(
            &mut changed_model
                .accepted
                .as_mut()
                .expect("fixture snapshot")
                .document,
        )
        .boards[0]
            .part_ids
            .clear();
        assert!(!layout_finding_return_is_current(
            &changed_model,
            &owner,
            Some(&outline_selection),
            &target,
        ));
    }

    #[wasm_bindgen_test]
    fn mounted_back_enter_focuses_board_and_outline_inspectors_and_rejects_stale_scope() {
        let (model, scope, selection) = fixture();
        let owner = LayoutOwnerIdentity {
            scope: Some(scope.clone()),
            token: Some(SnapshotToken(13)),
            revision: Some(9),
            generation: 2,
            workspace: "Layout",
        };
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document
            .create_element("div")
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap();
        root.set_id("m1-inspector-panel-content");
        document.body().unwrap().append_child(&root).unwrap();

        for (inspector, destination, expected_focus) in [
            (
                r#"<button id="back">Back to selection</button><section class="m1-board-inspector"><input id="board-name"></section>"#,
                objects::TreeContext::Board {
                    board_id: scope.board_id.clone(),
                },
                "board-name",
            ),
            (
                r#"<button id="back">Back to selection</button><section class="m1-outline-inspector"><select id="outline-version"><option>Generated</option></select></section>"#,
                objects::TreeContext::Outline {
                    board_id: scope.board_id.clone(),
                },
                "outline-version",
            ),
        ] {
            root.set_inner_html(inspector);
            let back = document
                .get_element_by_id("back")
                .unwrap()
                .dyn_into::<HtmlElement>()
                .unwrap();
            let target = LayoutFindingReturnTarget {
                owner: owner.clone(),
                selection: selection.clone(),
                destination: destination.clone(),
            };
            let current_selection = objects::ScopedTreeContext {
                scope: scope.clone(),
                context: destination,
            };
            let model = model.clone();
            let owner = owner.clone();
            let back_key = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event.key() == "Enter"
                    && layout_finding_return_is_current(
                        &model,
                        &owner,
                        Some(&current_selection),
                        &target,
                    )
                {
                    event.prevent_default();
                    let _ = focus_layout_finding_return_destination();
                }
            }) as Box<dyn FnMut(_)>);
            back.add_event_listener_with_callback("keydown", back_key.as_ref().unchecked_ref())
                .unwrap();
            back.focus().unwrap();
            let init = web_sys::KeyboardEventInit::new();
            init.set_key("Enter");
            init.set_bubbles(true);
            back.dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(
                document
                    .active_element()
                    .and_then(|element| element.get_attribute("id")),
                Some(expected_focus.to_owned()),
                "Enter returns focus into {expected_focus}"
            );
            back.remove_event_listener_with_callback("keydown", back_key.as_ref().unchecked_ref())
                .unwrap();
        }

        root.set_inner_html(
            r#"<button id="back">Back to selection</button><section class="m1-board-inspector"><input id="board-name"></section>"#,
        );
        let back = document
            .get_element_by_id("back")
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap();
        let target = LayoutFindingReturnTarget {
            owner: owner.clone(),
            selection,
            destination: objects::TreeContext::Board {
                board_id: scope.board_id.clone(),
            },
        };
        let current_selection = objects::ScopedTreeContext {
            scope,
            context: target.destination.clone(),
        };
        let mut stale_owner = owner.clone();
        stale_owner.scope.as_mut().expect("fixture scope").board_id = "other-board".into();
        let model = model.clone();
        let stale_key = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            if event.key() == "Enter"
                && layout_finding_return_is_current(
                    &model,
                    &stale_owner,
                    Some(&current_selection),
                    &target,
                )
            {
                event.prevent_default();
                let _ = focus_layout_finding_return_destination();
            }
        }) as Box<dyn FnMut(_)>);
        back.add_event_listener_with_callback("keydown", stale_key.as_ref().unchecked_ref())
            .unwrap();
        back.focus().unwrap();
        let init = web_sys::KeyboardEventInit::new();
        init.set_key("Enter");
        back.dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
        assert_eq!(
            document
                .active_element()
                .and_then(|element| element.get_attribute("id")),
            Some("back".to_owned()),
            "a stale scope cannot move focus into another board's Inspector"
        );
        back.remove_event_listener_with_callback("keydown", stale_key.as_ref().unchecked_ref())
            .unwrap();
        document.body().unwrap().remove_child(&root).unwrap();
    }
}

fn layout_finding_is_live(
    model: &ReadModel,
    request: &layout_findings::Request,
    destination_board: &str,
) -> bool {
    let Some(snapshot) = model.accepted.as_ref() else {
        return false;
    };
    if !keycaps_fit::presented_findings(&snapshot.scene.findings, &snapshot.document)
        .iter()
        .any(|finding| finding == &request.finding)
        || keycaps_fit::finding_navigation_target(&request.finding, &snapshot.document).as_ref()
            != Some(&request.target)
        || keycaps_fit::target_board_id(&request.target) != destination_board
    {
        return false;
    }
    snapshot
        .document
        .boards
        .iter()
        .any(|board| board.id == destination_board)
        && layout_finding_context(model, &request.target).is_some()
}

fn source_matches_layout_owner(
    source: &layout_findings::Source,
    owner: &LayoutOwnerIdentity,
    allow_board_hop: bool,
) -> bool {
    owner.workspace
        == if allow_board_hop {
            "Layout"
        } else {
            source.workspace
        }
        && owner.token == Some(source.token)
        && owner.revision == Some(source.revision)
        && owner.scope.as_ref().is_some_and(|scope| {
            scope.session_epoch == source.scope.session_epoch
                && scope.document_id == source.scope.document_id
                && (allow_board_hop || scope.board_id == source.scope.board_id)
        })
        && (allow_board_hop || owner.generation == source.generation)
}

fn findings_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    let model = runtime.model();
    owner.workspace != "Export"
        && current_layout_owner(runtime, workspace, adapter) == *owner
        && owner
            .scope
            .as_ref()
            .is_some_and(|scope| active_board_scope_matches(&model, scope))
}

struct LayoutFindingNavigationContext {
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    workspace: Signal<&'static str>,
    objects_open: Signal<bool>,
    inspect_open: Signal<bool>,
    findings_open: Signal<bool>,
    inspector_settings: Signal<PanelSettings>,
    focused_finding: Signal<Option<keycaps_finding_marker::FocusedFinding>>,
    svg: Rc<RefCell<Option<SvgElement>>>,
    alive: Rc<Cell<bool>>,
    body_selection: Signal<Option<case_viewer::BodySelection>>,
    case_selection: case_viewer::CaseSelection,
    select_tree: EventHandler<objects::TreeSelectRequest>,
    resumed_after_board_navigation: bool,
}

fn perform_layout_finding_navigation(
    context: LayoutFindingNavigationContext,
    request: layout_findings::Request,
) {
    let LayoutFindingNavigationContext {
        runtime,
        adapter,
        owner,
        workspace,
        mut objects_open,
        mut inspect_open,
        mut findings_open,
        inspector_settings,
        mut focused_finding,
        svg,
        alive,
        body_selection,
        case_selection,
        select_tree,
        resumed_after_board_navigation,
    } = context;
    if !layout_owner_is_current(&runtime, workspace, &adapter, &owner)
        || !source_matches_layout_owner(&request.source, &owner, resumed_after_board_navigation)
    {
        return;
    }
    let Some(scope) = owner.scope.as_ref() else {
        return;
    };
    let model = runtime.model();
    if !layout_finding_is_live(&model, &request, &scope.board_id) {
        return;
    }
    let Some(snapshot) = model.accepted.as_ref() else {
        return;
    };
    let Some(context) = layout_finding_context(&model, &request.target) else {
        return;
    };
    select_tree.call(objects::TreeSelectRequest {
        scope: scope.clone(),
        context: context.clone(),
        mode: SelectionMode::Replace,
        outline_action: None,
    });
    let route_model = runtime.model();
    let Some(route_snapshot) = route_model.accepted.as_ref() else {
        return;
    };
    if workspace() != "Layout"
        || runtime.scope().as_ref() != Some(scope)
        || route_snapshot.token != request.source.token
        || route_snapshot.document.revision != request.source.revision
    {
        return;
    }
    let route_generation = (adapter.generation)();

    let destination = keycaps_navigation::Destination::Layout(context);
    let navigation_owner = keycaps_navigation::NavigationOwner {
        workspace: "Layout",
        scope: scope.clone(),
        generation: route_generation,
        token: request.source.token,
        revision: request.source.revision,
        destination,
    };
    if let Some(surface) = svg.borrow().as_ref() {
        let rect = surface.get_bounding_client_rect();
        let surface_size = (rect.width(), rect.height());
        if let (Some(base), Some(target_bounds)) = (
            keycaps_fit::layout_camera_basis(
                &snapshot.document,
                &snapshot.scene,
                &scope.board_id,
                surface_size,
            ),
            keycaps_fit::finding_navigation_bounds(
                &snapshot.document,
                &snapshot.scene,
                &request.finding.id,
                &request.target,
            ),
        ) {
            let live_owner = keycaps_navigation::LiveNavigationOwner {
                workspace: "Layout",
                scope: Some(scope.clone()),
                generation: route_generation,
                token: Some(request.source.token),
                revision: Some(request.source.revision),
                destinations: vec![navigation_owner.destination.clone()],
            };
            if let Some(camera) = keycaps_navigation::destination_camera_fit(
                &navigation_owner,
                &live_owner,
                base.bounds,
                target_bounds,
                surface_size,
            ) {
                runtime.submit(Event::SetCamera {
                    operation_id: runtime.operation(),
                    center: camera.center,
                    zoom: camera.zoom,
                });
            }
        }
    }

    let navigation_id = keycaps_finding_marker::next_navigation_id(focused_finding.peek().as_ref());
    focused_finding.set(Some(keycaps_finding_marker::FocusedFinding {
        scope: scope.clone(),
        token: request.source.token,
        revision: request.source.revision,
        finding_id: request.finding.id,
        navigation_id,
    }));
    findings_open.set(false);
    objects_open.set(false);
    inspect_open.set(true);
    pin_inspector_on_desktop(inspector_settings);
    let focus_runtime = runtime.clone();
    let focus_adapter = adapter.clone();
    focus_first_inspector_control_on_next_frame(alive, navigation_owner, move || {
        current_keycaps_navigation_owner(
            workspace,
            &focus_runtime,
            &focus_adapter,
            body_selection,
            case_selection,
        )
    });
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

fn pcb_add_layout_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    if current_layout_owner(runtime, workspace, adapter) != *owner || owner.workspace != "PCB" {
        return false;
    }
    let model = runtime.model();
    let Some(scope) = owner.scope.as_ref() else {
        return false;
    };
    let (Some(token), Some(revision)) = (owner.token, owner.revision) else {
        return false;
    };
    active_board_scope_matches(&model, scope)
        && model.lifecycle == Lifecycle::Ready
        && model.durability == (Durability::Saved { revision })
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.token == token && snapshot.document.revision == revision
        })
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == scope.board_id)
        })
}

fn pcb_add_layout_open_handler(
    runtime: Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    assembly_3d: Signal<bool>,
    on_open: EventHandler<()>,
) -> EventHandler<()> {
    let mut workspace = workspace;
    let mut assembly_3d = assembly_3d;
    EventHandler::new(move |_| {
        if !pcb_add_layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
            return;
        }
        workspace.set("Layout");
        assembly_3d.set(false);
        on_open.call(());
    })
}

fn pcb_add_outline_select_handler(
    runtime: Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    assembly_3d: Signal<bool>,
    on_select: EventHandler<objects::TreeSelectRequest>,
) -> EventHandler<objects::TreeSelectRequest> {
    let mut workspace = workspace;
    let mut assembly_3d = assembly_3d;
    EventHandler::new(move |request: objects::TreeSelectRequest| {
        let objects::TreeContext::Outline { board_id } = &request.context else {
            on_select.call(request);
            return;
        };
        let Some(owner_scope) = owner.scope.as_ref() else {
            return;
        };
        if board_id != &owner_scope.board_id
            || request.scope != *owner_scope
            || !pcb_add_layout_owner_is_current(&runtime, workspace, &adapter, &owner)
        {
            return;
        }
        workspace.set("Layout");
        assembly_3d.set(false);
        on_select.call(request);
    })
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
    if model.selected_part_ids.is_empty()
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
    if model.selected_part_ids.iter().any(|selected_id| {
        !board.part_ids.iter().any(|id| id == selected_id)
            || !snapshot
                .document
                .parts
                .iter()
                .any(|part| part.id == *selected_id)
    }) {
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
    let is_matrix_member = document
        .matrices
        .iter()
        .any(|matrix| matrix.part_ids.contains(part_id));
    let layouts: Vec<_> = if is_matrix_member {
        Vec::new()
    } else {
        document
            .layouts
            .iter()
            .filter(|layout| layout.board_id == board.id)
            .map(|layout| inspector::LayoutChoice {
                id: layout.id.clone(),
                name: layout.name.clone(),
            })
            .collect()
    };
    let active_layout = document.layouts.iter().find(|layout| {
        layout.board_id == board.id
            && (layout.part_ids.contains(part_id)
                || document.matrices.iter().any(|matrix| {
                    matrix.id == layout.matrix_id && matrix.part_ids.contains(part_id)
                }))
    });
    let active_constraint = document
        .constraints
        .iter()
        .find(|constraint| {
            model
                .selected_part_ids
                .iter()
                .any(|id| constraint.target() == id)
                && board.part_ids.iter().any(|id| id == constraint.source())
        })
        .cloned();
    let relationship_summary =
        layout_relationship_summary(document, board, active_layout, Some(part));
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
            selected_part_ids: model.selected_part_ids.clone(),
        },
        reference: part.reference.clone(),
        definition_name: definition.name.clone(),
        definition_kind: definition_kind.to_owned(),
        envelope_notice: definition.envelope_notice.clone(),
        locked: model.selected_part_ids.iter().any(|selected_id| {
            document
                .parts
                .iter()
                .find(|part| part.id == *selected_id)
                .is_none_or(|part| part.locked.unwrap_or(false))
        }),
        position: part.pose.at,
        selection_count: model.selected_part_ids.len(),
        layout_id: active_layout.map(|layout| layout.id.clone()),
        layouts,
        outline: part.outline.clone().unwrap_or_else(PartOutline::default),
        board_parts,
        active_constraint,
        relationship_summary,
    })
}

fn matrix_context_relationship_summary(
    model: &ReadModel,
    selected: Option<&objects::ScopedTreeContext>,
) -> Option<String> {
    let selected = selected?;
    if !selection::context_is_current(model, &selected.scope, &selected.context) {
        return None;
    }
    let matrix_id = match &selected.context {
        objects::TreeContext::Matrix { matrix_id }
        | objects::TreeContext::Row { matrix_id, .. }
        | objects::TreeContext::Column { matrix_id, .. }
        | objects::TreeContext::Key { matrix_id, .. } => matrix_id,
        objects::TreeContext::Component {
            matrix_id: Some(matrix_id),
            ..
        } => matrix_id,
        _ => return None,
    };
    let snapshot = model.accepted.as_ref()?;
    let document = &snapshot.document;
    let board_id = &selected.scope.board_id;
    let layout = document
        .layouts
        .iter()
        .find(|layout| layout.board_id == *board_id && layout.matrix_id == *matrix_id);
    let board = document.boards.iter().find(|board| board.id == *board_id)?;
    Some(layout_relationship_summary(
        document,
        board,
        layout,
        matrix_context_active_part(model, selected),
    ))
}

fn matrix_context_relationship_target(
    model: &ReadModel,
    selected: Option<&objects::ScopedTreeContext>,
) -> Option<objects::TreeSelectRequest> {
    let selected = selected?;
    let part = matrix_context_active_part(model, selected)?;
    let context = objects::component_context_for_finding_part(model, &part.id)?;
    Some(objects::TreeSelectRequest {
        scope: selected.scope.clone(),
        context,
        mode: SelectionMode::Replace,
        outline_action: None,
    })
}

fn matrix_context_active_part<'a>(
    model: &'a ReadModel,
    selected: &objects::ScopedTreeContext,
) -> Option<&'a Part> {
    if !selection::context_is_current(model, &selected.scope, &selected.context) {
        return None;
    }
    let board = model
        .accepted
        .as_ref()?
        .document
        .boards
        .iter()
        .find(|board| board.id == selected.scope.board_id)?;
    let context_parts = objects::resolve_selection(model, &selected.context)?;
    model
        .selected_part_ids
        .iter()
        .find(|part_id| context_parts.contains(part_id) && board.part_ids.contains(part_id))
        .and_then(|part_id| {
            model
                .accepted
                .as_ref()?
                .document
                .parts
                .iter()
                .find(|part| part.id == *part_id)
        })
}

fn layout_relationship_summary(
    document: &boardstudio_core::model::ProjectDoc,
    board: &boardstudio_core::model::Board,
    active_layout: Option<&boardstudio_core::model::Layout>,
    active_part: Option<&Part>,
) -> String {
    let paired_layout = active_layout.and_then(|layout| {
        layout
            .mirror_link
            .as_ref()
            .and_then(|link| {
                document.layouts.iter().find(|candidate| {
                    candidate.board_id == board.id && candidate.id == link.source_id
                })
            })
            .or_else(|| {
                document.layouts.iter().find(|candidate| {
                    candidate.board_id == board.id
                        && candidate
                            .mirror_link
                            .as_ref()
                            .is_some_and(|link| link.source_id == layout.id)
                })
            })
    });
    if let Some(partner) = paired_layout {
        return format!(
            "Key assemblies, diodes and components mirror with {}. Replace a component on one half to keep it local.",
            partner.name
        );
    }
    if active_layout.is_some() {
        return "This layout is independent. Its geometry and components can be edited separately."
            .to_owned();
    }
    let driven = active_part.and_then(|target| {
        document
            .constraints
            .iter()
            .find(|constraint| {
                constraint.target() == target.id.as_str()
                    && board
                        .part_ids
                        .iter()
                        .any(|part_id| part_id == constraint.source())
            })
            .map(|constraint| {
                let source = document
                    .parts
                    .iter()
                    .find(|part| part.id == constraint.source())
                    .map(|part| part.reference.as_str())
                    .unwrap_or("A part");
                format!("{source} drives {}.", target.reference)
            })
    });
    driven.unwrap_or_else(|| "No saved placement relationship on this selection.".to_owned())
}

fn layout_selection_kind_for_tree_context(
    context: &objects::TreeContext,
) -> Option<objects::LayoutSelectionKind> {
    match context {
        objects::TreeContext::Matrix { .. } => Some(objects::LayoutSelectionKind::Matrix),
        objects::TreeContext::Row { .. } => Some(objects::LayoutSelectionKind::Row),
        objects::TreeContext::Column { .. } => Some(objects::LayoutSelectionKind::Column),
        objects::TreeContext::Key { .. } => Some(objects::LayoutSelectionKind::Key),
        objects::TreeContext::Component { .. } => Some(objects::LayoutSelectionKind::Part),
        objects::TreeContext::Outline { .. }
        | objects::TreeContext::OutlineVersion { .. }
        | objects::TreeContext::Bridge { .. }
        | objects::TreeContext::MountedModule { .. }
        | objects::TreeContext::Board { .. }
        | objects::TreeContext::LayoutGroup { .. } => None,
    }
}

fn update_layout_selection_kind_for_tree_context(
    mut selection_kind: Signal<objects::LayoutSelectionKind>,
    context: &objects::TreeContext,
) {
    if let Some(kind) = layout_selection_kind_for_tree_context(context) {
        selection_kind.set(kind);
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
        selected_part_ids: model.selected_part_ids.clone(),
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
        && model.selected_part_ids == owner.selected_part_ids
        && model.selected_part_ids.first() == Some(&owner.part_id)
}

/// Begin the pending edit for an Inspector field and park it on its field of the
/// Inspector's pending-edit state, so the form renders the draft while it is pending and
/// the accepted value again when it settles.
fn submit_layout_component_edit(
    runtime: &Rc<Runtime>,
    mut pending_edits: Signal<layout_component_edits::LayoutComponentInspectorEdits>,
    field: layout_component_edits::InspectorField,
    resolver: boardstudio_application::EditResolver,
) {
    let ticket = layout_component_edits::begin_inspector_edit(runtime, field, resolver);
    let mut edits = pending_edits.peek().clone();
    match field {
        layout_component_edits::InspectorField::X => edits.x = Some(ticket),
        layout_component_edits::InspectorField::Y => edits.y = Some(ticket),
        layout_component_edits::InspectorField::Margin => edits.margin = Some(ticket),
        layout_component_edits::InspectorField::Layout => edits.layout = Some(ticket),
        layout_component_edits::InspectorField::Constraint => edits.constraint = Some(ticket),
        layout_component_edits::InspectorField::RemoveConstraint => {
            edits.remove_constraint = Some(ticket)
        }
    }
    pending_edits.set(edits);
}

fn dispatch_layout_component_inspector_action(
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    layout_owner: &LayoutOwnerIdentity,
    lifetime: &inspector::LayoutComponentInspectorLifetime,
    mut workspace: Signal<&'static str>,
    mut inspect_open: Signal<bool>,
    pending_edits: Signal<layout_component_edits::LayoutComponentInspectorEdits>,
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
    if action_owner.selected_part_ids.is_empty()
        || action_owner.selected_part_ids.first() != Some(&action_owner.part_id)
        || action_owner.selected_part_ids.iter().any(|selected_id| {
            !board.part_ids.iter().any(|part_id| part_id == selected_id)
                || !snapshot
                    .document
                    .parts
                    .iter()
                    .any(|part| part.id == *selected_id)
        })
    {
        return;
    }
    match action {
        inspector::LayoutComponentInspectorAction::SetPosition { owner, axis, value } => {
            if !value.is_finite() {
                return;
            }
            let field = match axis {
                inspector::ComponentPositionAxis::X => layout_component_edits::InspectorField::X,
                inspector::ComponentPositionAxis::Y => layout_component_edits::InspectorField::Y,
            };
            submit_layout_component_edit(
                runtime,
                pending_edits,
                field,
                layout_component_edits::position_resolver(
                    owner.part_id.clone(),
                    owner.selected_part_ids.clone(),
                    owner.scope.board_id.clone(),
                    axis,
                    value,
                ),
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
            submit_layout_component_edit(
                runtime,
                pending_edits,
                layout_component_edits::InspectorField::Layout,
                layout_component_edits::assign_layout_resolver(
                    owner.part_id.clone(),
                    owner.scope.board_id.clone(),
                    layout_id,
                ),
            );
        }
        inspector::LayoutComponentInspectorAction::SetOutline { owner, outline } => {
            submit_layout_component_edit(
                runtime,
                pending_edits,
                layout_component_edits::InspectorField::Margin,
                layout_component_edits::outline_resolver(owner.part_id.clone(), outline),
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
            let id_seed = runtime.operation().0;
            submit_layout_component_edit(
                runtime,
                pending_edits,
                layout_component_edits::InspectorField::Constraint,
                layout_component_edits::constraint_resolver(
                    owner.part_id.clone(),
                    owner.scope.board_id.clone(),
                    source_part_id,
                    values,
                    id_seed,
                ),
            );
        }
        inspector::LayoutComponentInspectorAction::RemoveConstraint {
            owner,
            constraint_id,
        } => {
            submit_layout_component_edit(
                runtime,
                pending_edits,
                layout_component_edits::InspectorField::RemoveConstraint,
                layout_component_edits::remove_constraint_resolver(
                    owner.part_id.clone(),
                    constraint_id,
                ),
            );
        }
        inspector::LayoutComponentInspectorAction::NavigateElectrical { .. } => {
            workspace.set("PCB");
            inspect_open.set(true);
        }
    }
}

fn pending_splay_origin_pick_after_view_change<T>(
    from_3d: bool,
    to_3d: bool,
    pending: Option<T>,
) -> Option<T> {
    if !from_3d && to_3d { None } else { pending }
}

#[cfg(test)]
mod layout_splay_pick_view_change_tests {
    use super::pending_splay_origin_pick_after_view_change;
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn entering_layout_3d_cancels_pending_splay_origin_pick() {
        assert_eq!(
            pending_splay_origin_pick_after_view_change(false, true, Some("origin")),
            None
        );
        assert_eq!(
            pending_splay_origin_pick_after_view_change(true, false, Some("origin")),
            Some("origin"),
            "leaving 3D does not rewrite the pending pick state"
        );
    }
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
        | objects::TreeContext::MountedModule { .. }
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
        mounted_module_select: EventHandler::new(|_: String| {}),
        pcb_wiring_edit_board: EventHandler::new(|_: ()| {}),
        layout_selection_kind: EventHandler::new(|_: objects::LayoutSelectionKind| {}),
        layout_snap_intent: EventHandler::new(|_: objects::LayoutSnapIntent| {}),
        pcb_selection_kind: EventHandler::new(|_: objects::LayoutSelectionKind| {}),
        pcb_snap_intent: EventHandler::new(|_: objects::LayoutSnapIntent| {}),
        pcb_transform_properties: EventHandler::new(|_: ()| {}),
        case_action: EventHandler::new(|_: case_workspace::TreeAction| {}),
        case_display: EventHandler::new(|_: case_workspace::DisplayRequest| {}),
        keymap_layer: EventHandler::new(|_: String| {}),
        keymap_export: EventHandler::new(|_: ()| {}),
        show_configured_board: EventHandler::new(|_: String| {}),
        open_geometry_scripts: EventHandler::new(|_: ()| {}),
    });
    let runtime = use_context::<Rc<Runtime>>();
    let compact_panel_state = use_context::<CompactPanelState>();
    let mut objects_open = compact_panel_state.objects_open;
    let mut inspect_open = compact_panel_state.inspector_open;
    let mut geometry_scripts_open = use_signal(|| false);
    let preference_warning = use_context::<PreferenceStorageWarning>().0;
    let objects_panel_settings = use_panel_settings(PanelSide::Objects, preference_warning);
    let inspector_panel_settings = use_panel_settings(PanelSide::Inspector, preference_warning);
    let created_request_signal = use_context::<Signal<Option<SetupGuideRequest>>>();
    let created_request = created_request_signal();
    let mut guide_preferences = use_signal(|| None::<SetupGuidePreferences>);
    let mut guide_name_draft = use_signal(|| None::<(String, String)>);
    let mut guide_name_tickets =
        use_signal(Vec::<boardstudio_web_runtime::edit_ticket::EditTicket>::new);
    let mut guide_name_failure = use_signal(|| None::<String>);
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
            if let Some(value) = changed
                && guide_name_tickets.peek().is_empty()
            {
                guide_name_draft.set(Some(value));
            }
        }
    }));
    let adapter = use_context::<SelectionAdapter>();
    let layout_component_inspector_lifetime =
        use_hook(|| Rc::new(inspector::LayoutComponentInspectorLifetime::default()));
    let version = use_context::<Signal<u64>>();
    use_effect(use_reactive((&version(),), {
        let runtime = runtime.clone();
        move |_| {
            use boardstudio_web_runtime::edit_ticket::Settlement;
            let mut pending = guide_name_tickets.peek().clone();
            let had_tickets = !pending.is_empty();
            pending.retain(|ticket| match ticket.settlement(true) {
                Settlement::Pending => true,
                Settlement::Failed { message } => {
                    guide_name_failure.set(Some(message));
                    false
                }
                Settlement::Landed { .. } | Settlement::Retired => false,
            });
            if had_tickets && pending.is_empty() {
                guide_name_draft.set(runtime.model().accepted.as_ref().map(|snapshot| {
                    (snapshot.document.id.clone(), snapshot.document.name.clone())
                }));
            }
            if pending.len() != guide_name_tickets.peek().len() {
                guide_name_tickets.set(pending);
            }
        }
    }));

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
    let layout_findings_open = use_signal(|| false);
    let layout_finding_return_target = use_signal(|| None::<LayoutFindingReturnTarget>);
    let layout_finding_return_focus = use_signal(|| false);
    let pending_layout_finding = use_signal(|| None::<layout_findings::Request>);
    let pending_keycaps_navigation_fit =
        use_signal(|| None::<keycaps_navigation::PendingLayoutFit>);
    let keycaps_navigation_alive = keycaps_navigation::use_navigation_lifetime();
    let case_display = use_signal(std::collections::BTreeMap::new);
    let case_body_edit_dispatch = use_signal(|| None::<case_viewer::CaseBodyEditDispatch>);
    let case_body_editable = use_signal(|| false);
    let case_selection = case_viewer::CaseSelection {
        body: case_body_selection,
        layer: case_layer_selection,
        display: case_display,
        body_edit_portal: case_viewer::CaseBodyEditPortal {
            dispatch: case_body_edit_dispatch,
            editable: case_body_editable,
        },
    };
    use_context_provider(|| case_selection);
    let case_tree_expanded = use_signal(BTreeSet::<String>::new);
    let workspace = use_context::<WorkspaceState>().0;
    let return_workspace = use_context::<ExportReturnWorkspace>().0;
    let active_workspace = workspace();
    let requested_workspace_panel =
        panels::use_workspace_panel_defaults(active_workspace, objects_open, inspect_open);
    let render_generation = (adapter.generation)();
    let layout_selection_kind = use_signal(objects::LayoutSelectionKind::default);
    let layout_snap_settings = use_signal(objects::LayoutSnapSettings::default);
    let mut layout_context_tab = use_signal(layout_workspace::LayoutInspectorTab::default);
    layout_workspace::use_contextual_inspector_tab_reset(
        adapter.selected_context,
        layout_context_tab,
    );
    let layout_command_menu = use_signal(|| None::<objects::LayoutCommandMenu>);
    let mut layout_transform_tool = use_signal(|| None::<objects::LayoutTransformTool>);
    let layout_transform_tool_owner =
        use_signal(|| None::<(Option<Scope>, u64, Option<String>, &'static str, bool)>);
    let mut layout_assembly_3d = use_signal(|| false);
    let tree_cell_anchor = use_hook(|| Rc::new(RefCell::new(None::<OwnedTreeCellAnchor>)));
    let matrix_inspector = objects::use_matrix_inspector(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let board_inspector = board_inspector::use_board_inspector(
        runtime.clone(),
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
    let pending_splay_origin_pick = use_signal(|| None::<objects::MatrixTransformInspectorOwner>);
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
    let parts_assembly_orientation = use_signal(|| parts::SwitchOrientation::South);
    let canvas_interaction = use_hook(CanvasInteractionArbiter::default);
    let matrix_placement = objects::use_matrix_placement(
        runtime.clone(),
        objects::MatrixPlacementInput {
            version,
            selected_context: adapter.selected_context,
            anchor_scope: adapter.anchor_scope,
            workspace,
            scope_generation: adapter.generation,
            assembly_orientation: parts::PartsAssemblyOrientation(parts_assembly_orientation),
            canvas_interaction: canvas_interaction.clone(),
        },
    );
    let on_place_matrix_assembly = {
        let on_place = matrix_placement.on_place;
        let runtime = runtime.clone();
        let mut workspace = workspace;
        let mut selected_context = adapter.selected_context;
        let mut anchor_scope = adapter.anchor_scope;
        let mut layout_assembly_3d = layout_assembly_3d;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        let canvas_interaction = canvas_interaction.clone();
        EventHandler::new(move |source: objects::MatrixPlacementSource| {
            if workspace() != "Parts" || canvas_interaction.current().is_some() {
                return;
            }
            on_place.call(source);
            workspace.set("Layout");
            layout_assembly_3d.set(false);
            selected_context.set(None);
            anchor_scope.set(None);
            objects_open.set(false);
            inspect_open.set(false);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
        })
    };
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
    let parts_assembly_selection = use_signal(|| None);
    use_context_provider(|| parts::PartsAssemblySelection(parts_assembly_selection));
    use_context_provider(|| parts::PartsAssemblyOrientation(parts_assembly_orientation));
    let parts_selection_generation = use_signal(|| 0u64);
    use_context_provider(|| parts::PartsSelectionGeneration(parts_selection_generation));
    let parts_preview_activation = use_signal(|| 0u64);
    use_context_provider(|| parts::PartsPreviewActivation(parts_preview_activation));
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
    use_effect(use_reactive(
        (
            &layout_finding_return_target(),
            &layout_owner,
            &(adapter.selected_context)(),
        ),
        {
            let runtime = runtime.clone();
            let mut return_target = layout_finding_return_target;
            move |(target, owner, current_selection)| {
                let Some(target) = target.as_ref() else {
                    return;
                };
                if !layout_finding_return_is_current(
                    &runtime.model(),
                    &owner,
                    current_selection.as_ref(),
                    target,
                ) {
                    return_target.set(None);
                }
            }
        },
    ));
    use_effect(use_reactive((&layout_finding_return_focus(),), {
        let mut return_focus = layout_finding_return_focus;
        move |(pending,)| {
            if !pending {
                return;
            }
            return_focus.set(false);
            let _ = focus_layout_finding_return_destination();
        }
    }));
    use_effect(use_reactive((&active_workspace,), {
        let mut active_tool = layout_transform_tool;
        move |(active_workspace,)| {
            if active_workspace != "Layout" {
                active_tool.set(None);
            }
        }
    }));
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
            let adapter = adapter.clone();
            move |identity: &pcb_physical_setup::OwnerIdentity, strict: bool| {
                let model = runtime.model();
                let Some(accepted) = model.accepted.as_ref() else {
                    return false;
                };
                let accepted_transition_current =
                    identity.scope_transition.as_ref().is_none_or(|expected| {
                        identity.context == pcb_physical_setup::OwnerContext::CaseInspector
                            && !strict
                            && expected.matches_current(
                                adapter.scope_transition().as_ref(),
                                Some(accepted.token),
                                generation(),
                            )
                            && runtime.scope() == expected.next_scope
                    });
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
                accepted_transition_current
                    && context_current
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
    let encoder_input_actions = keymap::use_encoder_inputs(
        runtime.clone(),
        layer_source.clone(),
        pcb_wiring_mount.resolution_signal,
    );
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
            body_edit_portal: case_viewer::CaseBodyEditPortal {
                dispatch: case_body_edit_dispatch,
                editable: case_body_editable,
            },
        };
        let captured_revision = model
            .accepted
            .as_ref()
            .map(|snapshot| snapshot.document.revision);
        EventHandler::new(
            move |request: mechanical_settings_mount::MechanicalFindingNavigation| {
                let Some(scope) = captured_scope.as_ref() else {
                    return;
                };
                if request.scope != *scope
                    || Some(request.token) != captured_token
                    || Some(request.revision) != captured_revision
                {
                    return;
                }
                let model = runtime.model();
                if generation() != captured_generation
                    || runtime.scope().as_ref() != Some(scope)
                    || !instance_selection.is_current(&model)
                    || model.accepted.as_ref().is_none_or(|current| {
                        Some(current.token) != captured_token
                            || current.document.revision != request.revision
                    })
                {
                    return;
                }
                let Some(snapshot) = model.accepted.as_ref() else {
                    return;
                };
                let scene = runtime
                    .cad_scene()
                    .filter(|scene| &scene.scope == scope && Some(scene.token) == captured_token);
                let assembly = request
                    .resolution
                    .as_ref()
                    .map(|resolution| &resolution.assembly)
                    .or_else(|| scene.as_ref().and_then(|scene| scene.mechanical.as_ref()));
                let Some(assembly) = assembly else {
                    return;
                };
                let captured_document = &snapshot.document;
                let Some(finding) = assembly
                    .diagnostics
                    .iter()
                    .find(|finding| finding.id == request.finding_id)
                else {
                    return;
                };
                if let Some(layer) = assembly.stack.iter().find(|layer| {
                    finding.target_ids.contains(&layer.id)
                        || assembly.case.bodies.iter().any(|entry| {
                            entry.body.id == layer.id
                                && entry.body.mounts.as_ref().is_some_and(|mounts| {
                                    mounts
                                        .iter()
                                        .any(|mount| finding.target_ids.contains(&mount.id))
                                })
                        })
                }) {
                    case_selection.focus_layer(
                        scope.clone(),
                        request.token,
                        request.revision,
                        layer.id.clone(),
                    );
                    workspace.set("Case");
                } else if let Some(body) = captured_document.case_bodies.iter().find(|body| {
                    body.board_id == scope.board_id && finding.target_ids.contains(&body.id)
                }) {
                    case_selection.focus_layer(
                        scope.clone(),
                        request.token,
                        request.revision,
                        body.id.clone(),
                    );
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
            },
        )
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
            body_edit_portal: case_viewer::CaseBodyEditPortal {
                dispatch: case_body_edit_dispatch,
                editable: case_body_editable,
            },
        },
        on_show_mechanical_finding,
    );
    let Some(render_scope) = current_scope.clone() else {
        return rsx! {};
    };
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
                objects::TreeContext::MountedModule { .. }
                | objects::TreeContext::Outline { .. }
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
            selection::submit_context(&runtime, &adapter, request.into());
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
                }
                .into(),
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
                }
                .into(),
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
    let on_mounted_module_select = {
        let runtime = runtime.clone();
        let mut adapter = adapter.clone();
        let scope = render_scope.clone();
        let generation = render_generation;
        let token = snapshot.token;
        let revision = snapshot.document.revision;
        let mut parts_selection = parts_selection;
        let mut parts_selection_generation = parts_selection_generation;
        let mut parts_preview_activation = parts_preview_activation;
        let objects_open = objects_open;
        let mut inspect_open = inspect_open;
        move |module_id: String| {
            if !matches!(workspace(), "PCB" | "Layout")
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
            let Some(accepted) = model.accepted.as_ref().filter(|snapshot| {
                snapshot.token == token && snapshot.document.revision == revision
            }) else {
                return;
            };
            let Some(module) =
                accepted.document.modules.iter().find(|module| {
                    module.id == module_id && module.host_board_id == scope.board_id
                })
            else {
                return;
            };
            if !accepted
                .document
                .module_definitions
                .iter()
                .any(|definition| definition.id == module.definition_id)
            {
                return;
            }
            let module_definition_id = module.definition_id.clone();
            let context = objects::TreeContext::MountedModule {
                board_id: scope.board_id.clone(),
                module_id,
            };
            if !selection::context_is_current(&model, &scope, &context) {
                return;
            }
            adapter
                .selected_context
                .set(Some(objects::ScopedTreeContext {
                    scope: scope.clone(),
                    context,
                }));
            adapter.anchor_scope.set(None);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
            parts_selection.set(Some((
                Some(scope.clone()),
                format!("module:{module_definition_id}"),
            )));
            parts_selection_generation.with_mut(|value| *value = value.wrapping_add(1));
            parts_preview_activation.with_mut(|value| *value = value.wrapping_add(1));
            browse_parts_workspace(
                workspace,
                objects_open,
                objects_panel_settings,
                is_compact_viewport(),
            );
            inspect_open.set(true);
        }
    };
    let on_open_module_placement = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let scope = render_scope.clone();
        let generation = render_generation;
        let token = snapshot.token;
        let revision = snapshot.document.revision;
        let mut workspace = workspace;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        EventHandler::new(move |module_id: String| {
            if workspace() != "Parts"
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
            let current_context = adapter.selected_context.read().clone();
            let Some(current_context) = current_context.filter(|selected| {
                selected.scope == scope
                    && matches!(
                        &selected.context,
                        objects::TreeContext::MountedModule { board_id, module_id: selected_id }
                            if board_id == &scope.board_id && selected_id == &module_id
                    )
            }) else {
                return;
            };
            if !selection::context_is_current(&model, &scope, &current_context.context) {
                return;
            }
            let Some(accepted) = model.accepted.as_ref().filter(|snapshot| {
                snapshot.token == token && snapshot.document.revision == revision
            }) else {
                return;
            };
            let selected_definition = parts_selection().and_then(|(selected_scope, id)| {
                (selected_scope == Some(scope.clone()))
                    .then(|| id.strip_prefix("module:").map(str::to_owned))
                    .flatten()
            });
            if !accepted.document.modules.iter().any(|module| {
                module.id == module_id
                    && module.host_board_id == scope.board_id
                    && Some(module.definition_id.clone()) == selected_definition
            }) {
                return;
            }
            workspace.set("PCB");
            objects_open.set(false);
            inspect_open.set(true);
        })
    };
    let on_module_attached = {
        let runtime = runtime.clone();
        let mut adapter = adapter.clone();
        let mut workspace = workspace;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        EventHandler::new(move |request: parts::AttachedModuleNavigation| {
            if workspace() != "Parts"
                || runtime.scope().as_ref() != Some(&request.scope)
                || parts_selection_generation() != request.selection_generation
                || parts_selection()
                    != Some((
                        Some(request.scope.clone()),
                        format!("module:{}", request.definition_id),
                    ))
            {
                return;
            }
            let model = runtime.model();
            if !active_board_scope_matches(&model, &request.scope)
                || !instance_selection.is_current(&model)
            {
                return;
            }
            let Some(accepted) = model.accepted.as_ref().filter(|snapshot| {
                snapshot.token == request.snapshot_token
                    && snapshot.document.revision == request.revision
            }) else {
                return;
            };
            if !accepted.document.modules.iter().any(|module| {
                module.id == request.module_id
                    && module.definition_id == request.definition_id
                    && module.host_board_id == request.scope.board_id
            }) {
                return;
            }
            let context = objects::TreeContext::MountedModule {
                board_id: request.scope.board_id.clone(),
                module_id: request.module_id,
            };
            if !selection::context_is_current(&model, &request.scope, &context) {
                return;
            }
            adapter
                .selected_context
                .set(Some(objects::ScopedTreeContext {
                    scope: request.scope,
                    context,
                }));
            adapter.anchor_scope.set(None);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
            workspace.set("PCB");
            objects_open.set(false);
            inspect_open.set(true);
        })
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
                // Each press is a delta intent: it moves each part by one step from the
                // position accepted when it runs, so held-key repeats all land.
                let _ = boardstudio_web_runtime::edit_ticket::EditTicket::begin(
                    &runtime,
                    "layout-nudge",
                    Some("move".into()),
                    layout_component_edits::nudge_resolver(
                        moving_ids,
                        model.active_board_id.clone(),
                        Vec2 {
                            x: f64::from(request.dx) * step,
                            y: f64::from(request.dy) * step,
                        },
                    ),
                );
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
    let zoom_surface_size = use_signal(|| (1.0, 1.0));
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
    let keymap_surface = zoom_surface_size();
    // React normalizes its zoom against the physical getBounds basis; Keymap's existing camera
    // uses its key/contour basis. Convert only the displayed/effective scale between those bases.
    let keymap_scale_ratio = if active_workspace == "Keymap" && !layout_assembly_3d() {
        keycaps_fit::layout_canvas_bounds(&document, &scene, &render_scope.board_id)
            .map(|reference| {
                let reference = keycaps_fit::aspect_bounds(reference, keymap_surface);
                let current = keycaps_fit::aspect_bounds(
                    (
                        min_x,
                        min_x + (max_x - min_x).max(50.0),
                        min_y,
                        min_y + (max_y - min_y).max(50.0),
                    ),
                    keymap_surface,
                );
                ((reference.1 - reference.0) / (current.1 - current.0)).clamp(0.01, 100.0)
            })
            .filter(|ratio| ratio.is_finite())
            .unwrap_or(1.0)
    } else {
        1.0
    };
    let zoom_percent = model.camera.zoom * keymap_scale_ratio * 100.0;
    let width = (max_x - min_x).max(50.0) / model.camera.zoom;
    let height = (max_y - min_y).max(50.0) / model.camera.zoom;
    let view_x = (min_x + max_x - width) * 0.5 + model.camera.center.x;
    let view_y = -(min_y + max_y + height) * 0.5 - model.camera.center.y;
    let view_box = format!("{view_x} {view_y} {width} {height}");
    let footer_owner = layout_owner.clone();
    let footer_selected_ids = model.selected_part_ids.clone();
    let footer_camera_bounds = if active_workspace == "Layout" {
        // Layout's current camera basis already includes its 12 mm geometry margin and
        // aspect expansion; keymap::fit_camera adds 40 mm to its input to reconstruct
        // the mounted canvas base, so remove that symmetric padding here.
        (min_x + 20.0, max_x - 20.0, min_y + 20.0, max_y - 20.0)
    } else {
        workspace_rect_bounds.unwrap_or((-35.0, 35.0, -25.0, 25.0))
    };
    let footer_board_bounds = if active_workspace == "Keymap" {
        workspace_rect_bounds
    } else {
        keycaps_fit::layout_canvas_bounds(&document, &scene, &render_scope.board_id).map(
            |(min_x, max_x, min_y, max_y)| (min_x + 12.0, max_x - 12.0, min_y + 12.0, max_y - 12.0),
        )
    };
    let footer_selection_bounds = if active_workspace == "Keymap" {
        keymap_view.as_deref().and_then(|view| {
            keymap::selected_bounds(view, &footer_selected_ids.iter().cloned().collect())
        })
    } else {
        keycaps_fit::selected_part_bounds(
            &document,
            &scene,
            &render_scope.board_id,
            &footer_selected_ids,
        )
    };
    let footer_selection_available = footer_selection_bounds.is_some();
    let footer_svg = svg.clone();
    let footer_runtime = runtime.clone();
    let footer_workspace = workspace;
    let footer_adapter = adapter.clone();
    let on_fit_keymap_board = move |_| {
        if current_layout_owner(&footer_runtime, footer_workspace, &footer_adapter) != footer_owner
        {
            return;
        }
        let current = footer_runtime.model();
        let Some(scope) = footer_owner.scope.as_ref() else {
            return;
        };
        if !active_board_scope_matches(&current, scope) {
            return;
        }
        let target_bounds = footer_board_bounds.unwrap_or((-35.0, 35.0, -25.0, 25.0));
        let svg_ref = footer_svg.borrow();
        let Some(surface) = svg_ref.as_ref() else {
            return;
        };
        let rect = surface.get_bounding_client_rect();
        let Some(camera) = keymap::fit_camera(
            footer_camera_bounds,
            target_bounds,
            (rect.width(), rect.height()),
            keymap_toolbar_height(),
        ) else {
            return;
        };
        footer_runtime.submit(Event::SetCamera {
            operation_id: footer_runtime.operation(),
            center: camera.center,
            zoom: camera.zoom,
        });
    };
    let selection_owner = layout_owner.clone();
    let selection_runtime = runtime.clone();
    let selection_workspace = workspace;
    let selection_adapter = adapter.clone();
    let selection_svg = svg.clone();
    let selection_canvas_bounds = Some(footer_camera_bounds);
    let selection_ids = model.selected_part_ids.clone();
    let on_fit_keymap_selection = move |_| {
        if current_layout_owner(&selection_runtime, selection_workspace, &selection_adapter)
            != selection_owner
        {
            return;
        }
        let current = selection_runtime.model();
        if current.selected_part_ids != selection_ids {
            return;
        }
        let Some(scope) = selection_owner.scope.as_ref() else {
            return;
        };
        if !active_board_scope_matches(&current, scope) {
            return;
        }
        let Some(target_bounds) = footer_selection_bounds else {
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
    let zoom_runtime = runtime.clone();
    let zoom_workspace = workspace;
    let zoom_scope = render_scope.clone();
    let zoom_owner = layout_owner.clone();
    let zoom_adapter = adapter.clone();
    let zoom_svg = svg.clone();
    let zoom_token = snapshot.token;
    let zoom_revision = snapshot.document.revision;
    let zoom_ratio = keymap_scale_ratio;
    let zoom_bounds = bounds;
    let zoom_view = (view_x, view_y, width, height);
    let zoom_keymap = move |direction: f64| {
        let current = zoom_runtime.model();
        if current_layout_owner(&zoom_runtime, zoom_workspace, &zoom_adapter) != zoom_owner
            || zoom_runtime.scope().as_ref() != Some(&zoom_scope)
            || (zoom_adapter.generation)() != render_generation
            || current.active_board_id != zoom_scope.board_id
            || current.active_instance_id != zoom_scope.instance_id
        {
            return;
        }
        let Some(accepted) = current.accepted.as_ref() else {
            return;
        };
        if accepted.token != zoom_token
            || accepted.document.revision != zoom_revision
            || accepted.document.id != zoom_scope.document_id
            || accepted.session_epoch != zoom_scope.session_epoch
        {
            return;
        }
        let Some(window) = web_sys::window() else {
            return;
        };
        let (Ok(client_x), Ok(client_y)) = (window.inner_width(), window.inner_height()) else {
            return;
        };
        let (Some(client_x), Some(client_y)) = (client_x.as_f64(), client_y.as_f64()) else {
            return;
        };
        let surface_ref = zoom_svg.borrow();
        let Some(surface) = surface_ref.as_ref() else {
            return;
        };
        let rect = surface.get_bounding_client_rect();
        let (view_x, view_y, width, height) = zoom_view;
        let Some(location) = pointer_location(
            &rect,
            (client_x * 0.5).round() as i32,
            (client_y * 0.5).round() as i32,
            view_x,
            view_y,
            width,
            height,
        ) else {
            return;
        };
        let current_effective = current.camera.zoom * zoom_ratio;
        let next_effective = layout_camera::step_zoom(current_effective, direction);
        if (next_effective - current_effective).abs() < f64::EPSILON {
            return;
        }
        let next_zoom = next_effective / zoom_ratio;
        let center = zoom_center_at(zoom_bounds, location, next_zoom);
        zoom_runtime.submit(Event::SetCamera {
            operation_id: zoom_runtime.operation(),
            center,
            zoom: next_zoom,
        });
    };
    let zoom_out = zoom_keymap.clone();
    let on_zoom_keymap_out = move |_| zoom_out(-1.0);
    let on_zoom_keymap_in = move |_| zoom_keymap(1.0);
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
    let space_pan_window_listener = use_hook({
        let workspace = workspace;
        let space_down = space_down.clone();
        move || {
            let is_layout: Rc<dyn Fn() -> bool> = Rc::new(move || workspace() == "Layout");
            canvas_interaction::LayoutSpacePanWindowListener::install(is_layout, space_down.clone())
                .map(Rc::new)
        }
    });
    use_drop(move || drop(space_pan_window_listener));
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
    let on_toggle_layout_findings = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let mut findings_open = layout_findings_open;
        let mut inspect_open = inspect_open;
        let mut objects_open = objects_open;
        let inspector_settings = inspector_panel_settings;
        move |()| {
            if !findings_owner_is_current(&runtime, workspace, &adapter, &owner) {
                return;
            }
            findings_open.set(!findings_open());
            inspect_open.set(true);
            objects_open.set(false);
            pin_inspector_on_desktop(inspector_settings);
        }
    });
    let on_close_layout_findings = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let mut findings_open = layout_findings_open;
        let mut scripts_open = geometry_scripts_open;
        move |()| {
            if !findings_owner_is_current(&runtime, workspace, &adapter, &owner) {
                return;
            }
            findings_open.set(false);
            scripts_open.set(false);
            if let Some(element) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id("m1-layout-findings-trigger"))
                .and_then(|element| element.dyn_into::<HtmlElement>().ok())
            {
                let _ = element.focus();
            }
        }
    });
    let on_return_from_layout_finding = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let mut selected_tab = layout_context_tab;
        let select_tree = workspace_callbacks.select_tree;
        let mut findings_open = layout_findings_open;
        let mut return_target = layout_finding_return_target;
        let mut return_focus = layout_finding_return_focus;
        move |()| {
            let Some(target) = return_target.peek().clone() else {
                return;
            };
            if !findings_owner_is_current(&runtime, workspace, &adapter, &owner)
                || !layout_finding_return_is_current(
                    &runtime.model(),
                    &owner,
                    adapter.selected_context.read().as_ref(),
                    &target,
                )
            {
                return_target.set(None);
                return;
            }
            let Some(scope) = owner.scope.clone() else {
                return_target.set(None);
                return;
            };
            selected_tab.set(layout_workspace::LayoutInspectorTab::Properties);
            findings_open.set(false);
            select_tree.call(objects::TreeSelectRequest {
                scope,
                context: target.selection.context,
                mode: SelectionMode::Replace,
                outline_action: None,
            });
            return_target.set(None);
            return_focus.set(true);
        }
    });
    let on_layout_finding = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let objects_open = objects_open;
        let inspect_open = inspect_open;
        let findings_open = layout_findings_open;
        let inspector_settings = inspector_panel_settings;
        let mut pending = pending_layout_finding;
        let mut return_target = layout_finding_return_target;
        let focused_finding = focused_keycaps_finding;
        let svg = svg.clone();
        let alive = keycaps_navigation_alive.clone();
        let body_selection = case_body_selection;
        let select_tree = workspace_callbacks.select_tree;
        let navigate = workspace_callbacks.navigate;
        let mut active_workspace = workspace;
        move |request: layout_findings::Request| {
            if !findings_owner_is_current(&runtime, workspace, &adapter, &owner)
                || !source_matches_layout_owner(&request.source, &owner, false)
            {
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if !keycaps_fit::presented_findings(&snapshot.scene.findings, &snapshot.document)
                .iter()
                .any(|finding| finding == &request.finding)
                || keycaps_fit::finding_navigation_target(&request.finding, &snapshot.document)
                    .as_ref()
                    != Some(&request.target)
                || !layout_findings::target_has_live_layout_destination(
                    &request.target,
                    &snapshot.document,
                )
            {
                return;
            }
            let target_board = keycaps_fit::target_board_id(&request.target).to_owned();
            let Some(scope) = owner.scope.as_ref() else {
                return;
            };
            if target_board != scope.board_id {
                return_target.set(None);
                pending.set(Some(request));
                active_workspace.set("Layout");
                navigate.call((scope.clone(), target_board, None));
                return;
            }
            if owner.workspace != "Layout" {
                return_target.set(None);
                pending.set(Some(request));
                active_workspace.set("Layout");
                return;
            }
            let selected = adapter.selected_context.read().clone().filter(|selected| {
                selected.scope == *scope
                    && selection::context_is_current(&model, scope, &selected.context)
            });
            let Some(destination) = layout_finding_context(&model, &request.target) else {
                return;
            };
            return_target.set(selected.map(|selection| LayoutFindingReturnTarget {
                owner: owner.clone(),
                selection,
                destination,
            }));
            perform_layout_finding_navigation(
                LayoutFindingNavigationContext {
                    runtime: runtime.clone(),
                    adapter: adapter.clone(),
                    owner: owner.clone(),
                    workspace,
                    objects_open,
                    inspect_open,
                    findings_open,
                    inspector_settings,
                    focused_finding,
                    svg: svg.clone(),
                    alive: alive.clone(),
                    body_selection,
                    case_selection,
                    select_tree,
                    resumed_after_board_navigation: false,
                },
                request,
            );
        }
    });
    use_effect(use_reactive(
        (&pending_layout_finding(), &layout_owner, &active_workspace),
        {
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let objects_open = objects_open;
            let inspect_open = inspect_open;
            let findings_open = layout_findings_open;
            let inspector_settings = inspector_panel_settings;
            let mut pending = pending_layout_finding;
            let focused_finding = focused_keycaps_finding;
            let svg = svg.clone();
            let alive = keycaps_navigation_alive.clone();
            let body_selection = case_body_selection;
            let select_tree = workspace_callbacks.select_tree;
            move |(request, owner, active_workspace)| {
                let Some(request) = request else {
                    return;
                };
                if active_workspace != "Layout" {
                    if active_workspace != request.source.workspace {
                        pending.set(None);
                    }
                    return;
                }
                if !source_matches_layout_owner(&request.source, &owner, true) {
                    pending.set(None);
                    return;
                }
                let Some(scope) = owner.scope.as_ref() else {
                    pending.set(None);
                    return;
                };
                let target_board = keycaps_fit::target_board_id(&request.target);
                if scope.board_id != target_board {
                    if scope.board_id != request.source.scope.board_id {
                        pending.set(None);
                    }
                    return;
                }
                if request.source.workspace == "Layout"
                    && scope.board_id == request.source.scope.board_id
                {
                    return;
                }
                let model = runtime.model();
                if !layout_finding_is_live(&model, &request, target_board) {
                    pending.set(None);
                    return;
                }
                pending.set(None);
                perform_layout_finding_navigation(
                    LayoutFindingNavigationContext {
                        runtime: runtime.clone(),
                        adapter: adapter.clone(),
                        owner,
                        workspace,
                        objects_open,
                        inspect_open,
                        findings_open,
                        inspector_settings,
                        focused_finding,
                        svg: svg.clone(),
                        alive: alive.clone(),
                        body_selection,
                        case_selection,
                        select_tree,
                        resumed_after_board_navigation: true,
                    },
                    request,
                );
            }
        },
    ));
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
    let physical_setup_busy = physical_setup_mount.busy;
    let physical_setup_busy_value = physical_setup_busy();
    use_effect(use_reactive(
        (
            &observed_version,
            &preference_version,
            &observed_interaction_version,
            &physical_setup_busy_value,
        ),
        {
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let scope = render_scope.clone();
            let token = snapshot.token;
            let revision = snapshot.document.revision;
            let generation = render_generation;
            let drag = drag.clone();
            let physical_setup_busy = physical_setup_busy;
            let mut navigate_scoped = navigate_scoped.clone();
            move |_| {
                let model = runtime.model();
                if runtime.scope().as_ref() != Some(&scope)
                    || (adapter.generation)() != generation
                    || !instance_selection::can_reconcile(&model)
                    || physical_setup_busy.peek().is_some()
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
    let layout_transform_target = selected_tree_context
        .as_ref()
        .filter(|selected| selected.scope == render_scope && active_workspace == "Layout")
        .and_then(|selected| {
            let matrix_id = match &selected.context {
                objects::TreeContext::Matrix { matrix_id }
                | objects::TreeContext::Row { matrix_id, .. }
                | objects::TreeContext::Column { matrix_id, .. }
                | objects::TreeContext::Key { matrix_id, .. } => matrix_id.as_str(),
                objects::TreeContext::Component {
                    matrix_id: Some(matrix_id),
                    ..
                } => matrix_id.as_str(),
                _ => return None,
            };
            let matrix = matrices.iter().find(|matrix| matrix.id == matrix_id)?;
            let projection = matrix_scenes.get(matrix_id)?;
            Some((
                (**matrix).clone(),
                (**projection).clone(),
                selected.context.clone(),
            ))
        });
    let transform_tool_owner = (
        layout_owner.scope.clone(),
        layout_owner.generation,
        layout_transform_target
            .as_ref()
            .map(|(matrix, _, _)| matrix.id.clone()),
        active_workspace,
        layout_assembly_3d(),
    );
    use_effect(use_reactive((&transform_tool_owner,), {
        let mut owner_state = layout_transform_tool_owner;
        let mut active_tool = layout_transform_tool;
        move |(transform_tool_owner,)| {
            if owner_state
                .peek()
                .as_ref()
                .is_some_and(|previous| previous != &transform_tool_owner)
            {
                active_tool.set(None);
            }
            owner_state.set(Some(transform_tool_owner.clone()));
        }
    }));
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
    let board_inspector_projection = board_inspector.projection.clone().filter(|_| {
        component_inspector.is_none()
            && matrix_inspector.projection.is_none()
            && key_size.projection.is_none()
            && matrix_transform_inspector.projection.is_none()
            && outline_inspector.is_none()
    });
    let show_position_inspector = board_inspector_projection.is_none()
        && component_inspector.is_none()
        && selected_tree_context.as_ref().is_none_or(|selected| {
            let resolved = selection::resolve_context(&model, &selected.context);
            resolved.is_some_and(|ids| {
                ids.len() == 1 && model.selected_part_ids.as_slice() == ids.as_slice()
            }) && matches!(&selected.context, objects::TreeContext::Component { .. })
        });
    let context_summary = selected_tree_context
        .as_ref()
        .and_then(|selected| context_summary::summarize(&model, &selected.context));
    let mount = {
        let runtime = runtime.clone();
        let svg = svg.clone();
        let mut zoom_surface_size = zoom_surface_size;
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
                let rect = element.get_bounding_client_rect();
                zoom_surface_size.set((rect.width(), rect.height()));
                *svg.borrow_mut() = Some(element);
            }
        }
    };
    let placement_active = part_placement.projection.is_some()
        || part_placement.busy
        || matrix_placement.placement.is_some()
        || matrix_placement.busy;
    let canvas_aria_label = if matrix_placement.placement.is_some() || matrix_placement.busy {
        "Matrix placement canvas. Move the pointer or use arrow keys; click or press Enter to place, Escape to cancel."
    } else {
        "Keyboard layout; drag components, hold Shift for range selection, hold Space and drag to pan, or use position controls"
    };
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
        let matrix_placement = matrix_placement.clone();
        let tree_cell_anchor = tree_cell_anchor.clone();
        let mirrored_pair = mirrored_pair.clone();
        let canvas_interaction = canvas_interaction.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            if canvas_interaction.is_owner(CanvasInteractionOwner::MatrixPlacement) {
                let Some(active) = matrix_placement.placement.as_ref() else {
                    pointer.prevent_default();
                    pointer.stop_propagation();
                    return;
                };
                let owner = active.owner.clone();
                if workspace() != "Layout"
                    || runtime.scope().as_ref() != Some(&owner.scope)
                    || (adapter.generation)() != owner.scope_generation
                {
                    matrix_placement.on_cancel.call(owner);
                    return;
                }
                if let Some(center) = coordinates(&svg, &pointer, view_x, view_y, width, height) {
                    matrix_placement
                        .on_move
                        .call(objects::MatrixPlacementMove { owner, center });
                }
                return;
            }
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
                if !canvas_interaction::pending_part_drag_threshold_reached(
                    (current.client_x, current.client_y),
                    (f64::from(pointer.client_x()), f64::from(pointer.client_y())),
                ) {
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
        let matrix_placement = matrix_placement.clone();
        let snap_settings = layout_snap_settings;
        let render_scope = render_scope.clone();
        let canvas_interaction = canvas_interaction.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            if canvas_interaction.is_owner(CanvasInteractionOwner::MatrixPlacement) {
                pointer.prevent_default();
                pointer.stop_propagation();
                if pointer.button() == 0
                    && let Some(active) = matrix_placement.placement.as_ref()
                {
                    let owner = active.owner.clone();
                    if workspace() != "Layout"
                        || runtime.scope().as_ref() != Some(&owner.scope)
                        || (adapter.generation)() != owner.scope_generation
                    {
                        matrix_placement.on_cancel.call(owner);
                    } else if let Some(center) =
                        coordinates(&svg, &pointer, view_x, view_y, width, height)
                    {
                        matrix_placement
                            .on_commit
                            .call(objects::MatrixPlacementMove { owner, center });
                    }
                }
                return;
            }
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
        let matrix_placement = matrix_placement.clone();
        let render_scope = render_scope.clone();
        let canvas_interaction = canvas_interaction.clone();
        move |event: PointerEvent| {
            match canvas_interaction.current() {
                Some(CanvasInteractionOwner::MatrixPlacement) => {
                    if let Some(owner) = matrix_placement.cancel_owner.clone() {
                        matrix_placement.on_cancel.call(owner);
                    }
                    return;
                }
                Some(CanvasInteractionOwner::MirroredPair) => return,
                Some(CanvasInteractionOwner::PartPlacement) => {
                    if placement.projection.is_some() {
                        placement.on_cancel.call(());
                    }
                    return;
                }
                Some(CanvasInteractionOwner::OutlinePerimeter) => return,
                Some(CanvasInteractionOwner::MatrixTransform) => return,
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
        let matrix_placement = matrix_placement.clone();
        let canvas_interaction = canvas_interaction.clone();
        let snap_settings = layout_snap_settings;
        let mut pending_splay_pick = pending_splay_origin_pick;
        move |event: KeyboardEvent| {
            let key = event.data().key().to_string();
            let code = event.data().code().to_string();
            let modifiers = event.data().modifiers();
            if canvas_interaction.is_owner(CanvasInteractionOwner::MatrixPlacement) {
                let Some(active) = matrix_placement.placement.as_ref() else {
                    if key == "Escape"
                        && let Some(owner) = matrix_placement.cancel_owner.clone()
                    {
                        event.prevent_default();
                        matrix_placement.on_cancel.call(owner);
                    } else {
                        event.prevent_default();
                    }
                    return;
                };
                let owner = active.owner.clone();
                if key == "Escape" {
                    event.prevent_default();
                    matrix_placement.on_cancel.call(owner);
                    return;
                }
                if key == "Enter" {
                    event.prevent_default();
                    matrix_placement
                        .on_commit
                        .call(objects::MatrixPlacementMove {
                            owner,
                            center: active.matrix.origin,
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
                    let fraction = snap_settings.read().snap_fraction;
                    let step = |pitch: f64| {
                        if fraction.is_sign_negative() {
                            -fraction
                        } else {
                            pitch * if fraction == 0.0 { 0.25 } else { fraction }
                        }
                    };
                    matrix_placement.on_move.call(objects::MatrixPlacementMove {
                        owner,
                        center: Vec2 {
                            x: active.matrix.origin.x + dx * step(active.matrix.pitch.x),
                            y: active.matrix.origin.y + dy * step(active.matrix.pitch.y),
                        },
                    });
                    return;
                }
            }
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
            if key == "Escape" && pending_splay_pick.peek().is_some() {
                pending_splay_pick.set(None);
                event.prevent_default();
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
        let mut pending_splay_pick = pending_splay_origin_pick;
        let transform_inspector = matrix_transform_inspector.clone();
        let layout_owner_for_pick = layout_owner.clone();
        move |event: PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            if canvas_interaction.is_owner(CanvasInteractionOwner::MatrixPlacement) {
                pointer.prevent_default();
                pointer.stop_propagation();
                return;
            }
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
            if pointer.button() == 0
                && workspace() == "Layout"
                && pending_splay_pick.peek().is_some()
                && let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height)
            {
                pointer.prevent_default();
                pointer.stop_propagation();
                apply_pending_splay_origin_pick(
                    &runtime,
                    workspace,
                    &adapter,
                    &layout_owner_for_pick,
                    &mut pending_splay_pick,
                    &transform_inspector,
                    point,
                );
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
            // Modified hits change selection only, matching the reference.
            if mode != SelectionMode::Replace {
                return;
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
            // keymap_scale_ratio is 1.0 outside Keymap, so Layout and Keymap share the
            // reference 0.25..4 limits.
            let zoom = layout_camera::wheel_zoom(old.zoom, keymap_scale_ratio, wheel.delta_y());
            let center = zoom_center_at((min_x, max_x, min_y, max_y), location, zoom);
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
        .mounted_module_select
        .replace(Box::new(on_mounted_module_select));
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
    {
        let mut workspace = workspace;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        let mut geometry_scripts_open = geometry_scripts_open;
        let inspector_settings = inspector_panel_settings;
        workspace_callbacks
            .open_geometry_scripts
            .replace(Box::new(move |_| {
                workspace.set("Layout");
                objects_open.set(false);
                inspect_open.set(true);
                geometry_scripts_open.set(true);
                pin_inspector_on_desktop(inspector_settings);
            }));
    }
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
    let board_setup = objects::use_board_setup(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        workspace_callbacks.navigate,
    );
    let shared_objects = workspace_composition::SharedObjectsInput {
        selected_context: adapter.selected_context,
        on_select: workspace_callbacks.select_tree,
        on_navigate: workspace_callbacks.navigate,
        on_nudge: workspace_callbacks.nudge_tree,
        on_open_geometry_scripts: workspace_callbacks.open_geometry_scripts,
        board_setup,
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
    let on_empty_board_matrix = use_callback({
        let runtime = runtime.clone();
        let owner_scope = render_scope.clone();
        let owner_token = snapshot.token;
        let owner_revision = snapshot.document.revision;
        let generation = adapter.generation;
        let mut workspace = workspace;
        let on_open = matrix_setup.on_open;
        move |_| {
            let model = runtime.model();
            if workspace() != active_workspace
                || generation() != render_generation
                || runtime.scope().as_ref() != Some(&owner_scope)
                || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: owner_revision,
                    })
                || model.gesture.is_some()
                || model.display_preview.is_some()
                || !model.accepted.as_ref().is_some_and(|accepted| {
                    accepted.token == owner_token && accepted.document.revision == owner_revision
                })
            {
                return;
            }
            workspace.set("Layout");
            let mut layout_assembly_3d = layout_assembly_3d;
            layout_assembly_3d.set(false);
            on_open.call(());
        }
    });
    let objects_input = match active_workspace {
        "PCB" => {
            let pcb_add_owner = LayoutOwnerIdentity {
                scope: Some(render_scope.clone()),
                token: Some(snapshot.token),
                revision: Some(snapshot.document.revision),
                generation: render_generation,
                workspace: "PCB",
            };
            let pcb_add_is_current =
                pcb_add_layout_owner_is_current(&runtime, workspace, &adapter, &pcb_add_owner);
            let mut pcb_shared_objects = shared_objects;
            pcb_shared_objects.on_select = pcb_add_outline_select_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                pcb_add_owner.clone(),
                layout_assembly_3d,
                pcb_shared_objects.on_select,
            );
            pcb_shared_objects.on_open_geometry_scripts = pcb_add_layout_open_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                pcb_add_owner.clone(),
                layout_assembly_3d,
                pcb_shared_objects.on_open_geometry_scripts,
            );
            let mut pcb_matrix_setup = matrix_setup.clone();
            pcb_matrix_setup.can_open = pcb_add_is_current
                && pcb_matrix_setup.projection.is_none()
                && canvas_interaction.current().is_none();
            pcb_matrix_setup.on_open = pcb_add_layout_open_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                pcb_add_owner.clone(),
                layout_assembly_3d,
                matrix_setup.on_open,
            );
            let mut pcb_mirrored_pair = mirrored_pair.clone();
            pcb_mirrored_pair.can_open = pcb_add_is_current
                && pcb_mirrored_pair.form.is_none()
                && pcb_mirrored_pair.placement.is_none()
                && !pcb_mirrored_pair.owns_canvas
                && canvas_interaction.current().is_none();
            pcb_mirrored_pair.on_open = pcb_add_layout_open_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                pcb_add_owner,
                layout_assembly_3d,
                mirrored_pair.on_open,
            );
            workspace_composition::WorkspaceObjectsInput::Pcb(Box::new(
                pcb_workspace::ObjectsInput {
                    shared: pcb_shared_objects,
                    matrix_setup: pcb_matrix_setup,
                    mirrored_pair: pcb_mirrored_pair,
                    on_place_component: part_placement.on_place_component,
                    layout_target,
                    parts_query,
                    on_browse_parts,
                    placement_error: matrix_placement
                        .error
                        .clone()
                        .or_else(|| part_placement.error.clone()),
                },
            ))
        }
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
                matrix_inspector: matrix_inspector.clone(),
                mirrored_pair: mirrored_pair.clone(),
                pair_created: pair_created_selection,
                on_place_component: part_placement.on_place_component,
                layout_target,
                parts_query,
                on_browse_parts,
                placement_error: matrix_placement
                    .error
                    .clone()
                    .or_else(|| part_placement.error.clone()),
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
        {
            let mut active_tool = layout_transform_tool;
            move |assembly_3d| {
                active_tool.set(None);
                layout_assembly_3d.set(assembly_3d);
            }
        },
        LayoutPlacementCancellation {
            parts: part_placement.clone(),
            matrices: matrix_placement.clone(),
            interactions: canvas_interaction.clone(),
        },
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
            let matrix_placement = matrix_placement.clone();
            let mut pending_splay_origin_pick = pending_splay_origin_pick;
            let layout_assembly_3d = layout_assembly_3d;
            move || {
                pending_splay_origin_pick.set(pending_splay_origin_pick_after_view_change(
                    layout_assembly_3d(),
                    true,
                    pending_splay_origin_pick(),
                ));
                if let Some(active) = mirrored_pair.placement.as_ref() {
                    mirrored_pair.on_cancel.call(active.owner.clone());
                } else if let Some(form) = mirrored_pair.form.as_ref() {
                    mirrored_pair.on_cancel.call(form.owner.clone());
                }
                if let Some(projection) = matrix_setup.projection.as_ref() {
                    matrix_setup.on_cancel.call(projection.owner.clone());
                }
                if let Some(owner) = matrix_placement.cancel_owner.clone() {
                    matrix_placement.on_cancel.call(owner);
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
                let mut layout_context_tab = layout_context_tab;
                move |_| {
                    if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
                        return;
                    }
                    pin_inspector_on_desktop(inspector_panel_settings);
                    layout_context_tab.set(layout_workspace::LayoutInspectorTab::Properties);
                    objects_open.set(false);
                    inspect_open.set(true);
                }
            });
            let pointer_tools_available = layout_transform_target.is_some();
            let on_transform_tool = EventHandler::new({
                let runtime = runtime.clone();
                let adapter = adapter.clone();
                let owner = layout_owner.clone();
                let mut active_tool = layout_transform_tool;
                let on_selection_kind = workspace_callbacks.layout_selection_kind;
                let matrix_ids: BTreeSet<_> =
                    matrices.iter().map(|matrix| matrix.id.clone()).collect();
                move |tool| {
                    if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
                        return;
                    }
                    let Some(selected) =
                        adapter.selected_context.read().clone().filter(|selected| {
                            owner.scope.as_ref() == Some(&selected.scope)
                                && selection::context_is_current(
                                    &runtime.model(),
                                    &selected.scope,
                                    &selected.context,
                                )
                        })
                    else {
                        return;
                    };
                    let matrix_id = match &selected.context {
                        objects::TreeContext::Matrix { matrix_id }
                        | objects::TreeContext::Row { matrix_id, .. }
                        | objects::TreeContext::Column { matrix_id, .. }
                        | objects::TreeContext::Key { matrix_id, .. } => matrix_id.as_str(),
                        objects::TreeContext::Component {
                            matrix_id: Some(matrix_id),
                            ..
                        } => matrix_id.as_str(),
                        _ => return,
                    };
                    if !matrix_ids.contains(matrix_id) || active_tool() == Some(tool) {
                        active_tool.set(None);
                        return;
                    }
                    active_tool.set(Some(tool));
                    let next_kind = match (tool, &selected.context) {
                        (
                            objects::LayoutTransformTool::Stagger,
                            objects::TreeContext::Row { .. },
                        ) => objects::LayoutSelectionKind::Row,
                        _ => objects::LayoutSelectionKind::Column,
                    };
                    on_selection_kind.call(next_kind);
                }
            });
            let transform = objects::LayoutTransformMenuMount {
                properties_available,
                column_available: supports_kind(objects::LayoutSelectionKind::Column),
                row_available: supports_kind(objects::LayoutSelectionKind::Row),
                pointer_tools_visible: true,
                pointer_tools_available,
                active_tool: layout_transform_tool(),
                on_selection_kind: workspace_callbacks.layout_selection_kind,
                on_show_properties,
                on_transform_tool,
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
                    open_menu: layout_command_menu,
                    on_selection_kind: workspace_callbacks.layout_selection_kind,
                    on_snap_intent: workspace_callbacks.layout_snap_intent,
                    show_relationships: true,
                    has_selection_context: matrix_transform_inspector.projection.is_some()
                        || component_inspector.is_some(),
                    on_show_relationships: EventHandler::new(move |()| {
                        layout_context_tab.set(layout_workspace::LayoutInspectorTab::Relations);
                        inspect_open.set(true);
                    }),
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
            }) && show_position_inspector;
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
                pointer_tools_visible: false,
                pointer_tools_available: false,
                active_tool: None,
                on_selection_kind: workspace_callbacks.pcb_selection_kind,
                on_show_properties: workspace_callbacks.pcb_transform_properties,
                on_transform_tool: EventHandler::new(|_| {}),
            };
            workspace_composition::WorkspaceToolbarInput::Pcb(Box::new(
                pcb_workspace::ToolbarInput {
                    command_label: "PCB commands".to_owned(),
                    menu_owner_key: format!("{layout_owner:?}"),
                    open_menu: layout_command_menu,
                    selection_kind: layout_selection_kind(),
                    snap_settings: layout_snap_settings.read().clone(),
                    transform,
                    align: pcb_align.clone(),
                    on_selection_kind: workspace_callbacks.pcb_selection_kind,
                    on_snap_intent: workspace_callbacks.pcb_snap_intent,
                    show_relationships: false,
                    has_selection_context: false,
                    on_show_relationships: EventHandler::new(|()| {}),
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
                on_module_select: workspace_callbacks.mounted_module_select,
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
    let component_inspector_pending_edits =
        use_signal(layout_component_edits::LayoutComponentInspectorEdits::default);
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
                component_inspector_pending_edits,
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
                selected_part_summary: case_workspace::selected_part_summary(
                    &model,
                    selected_tree_context.as_ref(),
                ),
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
                placement_busy: part_placement.busy || matrix_placement.busy,
                placement_error: matrix_placement
                    .error
                    .clone()
                    .or_else(|| part_placement.error.clone()),
                layout_target,
                on_place_assembly: on_place_matrix_assembly,
                on_board_placed: EventHandler::new(move |_| {
                    let mut selection_kind = layout_selection_kind;
                    selection_kind.set(objects::LayoutSelectionKind::Part);
                }),
                on_open_module_placement,
                on_module_attached,
            },
        )),
        "PCB" => {
            let selected_module = selected_tree_context.as_ref().and_then(|selected| {
                if selected.scope != render_scope {
                    return None;
                }
                match &selected.context {
                    objects::TreeContext::MountedModule {
                        board_id,
                        module_id,
                    } if board_id == &render_scope.board_id => Some(module_id.clone()),
                    _ => None,
                }
            });
            if let Some(module_id) = selected_module {
                workspace_composition::WorkspaceInspectorInput::PcbModule(Box::new(
                    pcb_module_inspector::InspectorInput {
                        runtime: runtime.clone(),
                        snapshot: snapshot.clone(),
                        scope: render_scope.clone(),
                        module_id,
                        selected_context: adapter.selected_context,
                        on_place_component: part_placement.on_place_component,
                    },
                ))
            } else {
                workspace_composition::WorkspaceInspectorInput::Pcb(pcb_wiring_source.map(
                    |source| {
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
                        })
                    },
                ))
            }
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
                geometry_scripts_open: geometry_scripts_open(),
                on_close_geometry_scripts: EventHandler::new(move |()| {
                    geometry_scripts_open.set(false);
                }),
                context_title: context_summary
                    .as_ref()
                    .map(|summary| summary.title.clone())
                    .or_else(|| {
                        board_inspector_projection
                            .as_ref()
                            .map(|board| board.board_name.clone())
                    }),
                context_detail: context_summary
                    .as_ref()
                    .and_then(|summary| summary.detail.clone())
                    .or_else(|| {
                        board_inspector_projection
                            .as_ref()
                            .map(|_| "Layout".to_owned())
                    }),
                show_position_inspector,
                component_inspector: component_inspector.clone(),
                component_inspector_pending_edits,
                on_component_inspector_action,
                matrix_inspector,
                key_size,
                matrix_transform_inspector: matrix_transform_inspector.clone(),
                on_pick_splay_origin: {
                    let mount = matrix_transform_inspector.clone();
                    let mut pending = pending_splay_origin_pick;
                    EventHandler::new(move |()| {
                        let Some(projection) = mount.projection.as_ref() else {
                            return;
                        };
                        if matches!(
                            &projection.owner.context,
                            objects::TreeContext::Column { .. }
                        ) && mount.editable
                        {
                            pending.set(Some(projection.owner.clone()));
                        }
                    })
                },
                splay_origin_pick_pending: pending_splay_origin_pick().is_some(),
                on_cancel_splay_origin_pick: {
                    let mut pending = pending_splay_origin_pick;
                    EventHandler::new(move |()| pending.set(None))
                },
                inspector_tab: layout_context_tab,
                matrix_relationship_summary: matrix_context_relationship_summary(
                    &model,
                    selected_tree_context.as_ref(),
                ),
                matrix_relationship_target: matrix_context_relationship_target(
                    &model,
                    selected_tree_context.as_ref(),
                ),
                on_select_context: workspace_callbacks.select_tree,
                outline_inspector: outline_inspector.clone().map(Box::new),
                findings_return_available: layout_finding_return_target().as_ref().is_some_and(
                    |target| {
                        layout_finding_return_is_current(
                            &model,
                            &layout_owner,
                            selected_tree_context.as_ref(),
                            target,
                        )
                    },
                ),
                on_findings_return: on_return_from_layout_finding,
                board_inspector: board_inspector_projection,
                on_board_rename: board_inspector.on_rename,
                findings_page: Some(layout_findings::InspectorMount {
                    open: layout_findings_open(),
                    document: Rc::new(document.as_ref().clone()),
                    findings: snapshot.scene.findings.clone(),
                    source: layout_findings::Source {
                        workspace: active_workspace,
                        scope: render_scope.clone(),
                        token: snapshot.token,
                        revision: snapshot.document.revision,
                        generation: render_generation,
                    },
                    on_close: on_close_layout_findings,
                    on_navigate: on_layout_finding,
                }),
            },
        )),
    };
    let board_reference_editor = if matches!(active_workspace, "Layout" | "Case") {
        let reference = document
            .board_references
            .iter()
            .find(|reference| reference.board_id == render_scope.board_id)
            .cloned();
        let owner = LayoutOwnerIdentity {
            scope: Some(render_scope.clone()),
            token: Some(snapshot.token),
            revision: Some(snapshot.document.revision),
            generation: render_generation,
            workspace: active_workspace,
        };
        let assets = document.assets.clone();
        let editable = boardstudio_web_pcb::board_reference_owner::board_reference_owner_is_current(
            &runtime, workspace, &adapter, &owner,
        );
        Some((reference, assets, editable, owner))
    } else {
        None
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
        || matrix_placement.placement.is_some()
        || matrix_placement.busy
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
                    || matrix_placement.placement.is_some()
                    || matrix_placement.busy
                    || mirrored_pair.form.is_some()
                    || mirrored_pair.placement.is_some()))
            && !controller_guide_hidden
    });
    let show_empty_board = guide.is_none()
        && matches!(active_workspace, "Layout" | "PCB" | "Keymap" | "Keycaps")
        && !layout_assembly_3d()
        && visible.is_empty()
        && matrices.is_empty()
        && !scene.board_contours.iter().any(|contours| {
            contours.board_id == model.active_board_id && !contours.contours.is_empty()
        })
        && mirrored_pair.placement.is_none()
        && part_placement.projection.is_none()
        && !part_placement.busy;
    let show_empty_board =
        show_empty_board && matrix_placement.placement.is_none() && !matrix_placement.busy;
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
    let outline_grid_active = outline_inspector
        .as_ref()
        .is_some_and(|projection| (projection.editing_points)());
    let canvas_grid = objects::layout_canvas_grid_style(
        width,
        zoom_surface_size().0,
        outline_pitch.x,
        outline_snap_settings.snap_fraction,
        outline_grid_active,
    );
    let outline_overlay_key = outline_inspector
        .as_ref()
        .map(outline_lifecycle::OutlineInspectorProjection::canvas_edit_key);
    let name_value = guide_name_draft()
        .filter(|(project_id, _)| project_id == &document.id)
        .map(|(_, name)| name)
        .unwrap_or_else(|| document.name.clone());
    let name_project_id = document.id.clone();
    let on_name_change = move |name: String| {
        guide_name_failure.set(None);
        guide_name_draft.set(Some((name_project_id.clone(), name)));
    };
    let name_runtime = runtime.clone();
    let mut name_draft = guide_name_draft;
    let expected_document_id = document.id.clone();
    let expected_epoch = snapshot.session_epoch;
    let on_name_commit = move |_| {
        let Some(current) = name_runtime.model().accepted else {
            return;
        };
        if current.session_epoch != expected_epoch || current.document.id != expected_document_id {
            return;
        }
        let proposed = name_draft()
            .filter(|(id, _)| id == &expected_document_id)
            .map(|(_, value)| value.trim().to_owned())
            .unwrap_or_default();
        if proposed.is_empty() {
            name_draft.set(Some((
                current.document.id.clone(),
                current.document.name.clone(),
            )));
            return;
        }
        let target_document_id = expected_document_id.clone();
        let resolver = boardstudio_application::EditResolver::new(
            "setup-project-name",
            move |accepted: &boardstudio_application::AcceptedSnapshot| {
                if accepted.session_epoch != expected_epoch
                    || accepted.document.id != target_document_id
                {
                    return boardstudio_application::Resolution::Retire(
                        "The project is no longer open.".into(),
                    );
                }
                if accepted.document.name == proposed {
                    return boardstudio_application::Resolution::Unchanged;
                }
                let mut document = accepted.document.as_ref().clone();
                document.name = proposed.clone();
                boardstudio_application::Resolution::Submit(EditCommand {
                    base_revision: 0,
                    transaction_id: String::new(),
                    phase: EditPhase::Commit,
                    target_ids: vec![document.id.clone()],
                    operation: EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                })
            },
        );
        guide_name_tickets
            .write()
            .push(boardstudio_web_runtime::edit_ticket::EditTicket::begin(
                &name_runtime,
                "setup-project-name",
                Some("project name".into()),
                resolver,
            ));
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
    let guide_statuses = setup_guide::stage_statuses(
        &document,
        &model.active_board_id,
        &snapshot.scene,
        match &pcb_wiring_mount.resolution {
            pcb_wiring::PcbWiringResolution::Current { identity, plan }
                if identity.scope.document_id == document.id
                    && identity.scope.board_id == model.active_board_id
                    && identity.scope.instance_id.is_none()
                    && identity.token == snapshot.token
                    && identity.revision == document.revision =>
            {
                Some(plan.as_ref())
            }
            _ => None,
        },
    );
    let guide_stage_index = guide
        .as_ref()
        .map(|preferences| match preferences.current_stage {
            SetupGuideStage::Project => 0,
            SetupGuideStage::Layout => 1,
            SetupGuideStage::Wiring => 2,
            SetupGuideStage::Case => 3,
            SetupGuideStage::Review => 4,
        })
        .unwrap_or(0);
    let guide_stage_readiness = guide_statuses.clone().map(|status| status.ready);
    let guide_stage_detail = guide_statuses[guide_stage_index].detail.clone();
    rsx! {
        section { class: "m1-editor", "aria-label": "Keyboard editor",
            panels::CompactPanelScrim {}
            div { class: "m1-editor-body", style: "{panel_layout_style}",
                ObjectsPanel { compact_open: objects_open, settings: objects_panel_settings,
                    if let Some(preferences) = guide {
                        if let Some(message) = guide_name_failure() { p { role: "alert", "{message}" } }
                        setup_guide::ProjectSetupGuide {
                            stage: preferences.current_stage,
                            stage_readiness: guide_stage_readiness,
                            stage_detail: guide_stage_detail,
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
                        layout_viewer::LayoutCanonicalViewer {
                            on_mounted_module_pick: (active_workspace == "Layout")
                                .then_some(workspace_callbacks.mounted_module_select),
                            keycaps_fit: if matches!(active_workspace, "Keymap" | "Keycaps") {
                                keycaps_fit_state.state.clone()
                            } else {
                                None
                            },
                            focused_finding: if active_workspace == "Layout" {
                                focused_keycaps_finding()
                            } else {
                                None
                            },
                        }
                    } else if active_workspace == "Layout" {
                    svg { class: "m1-canvas", view_box: "{view_box}", preserve_aspect_ratio: "xMidYMid meet", tabindex: "0", role: "group", "aria-label": "{canvas_aria_label}", onmounted: mount,
                    onpointerdown: start_pan, onpointermove: move_pointer, onpointerup: end_pointer, onpointercancel: cancel_pointer.clone(), onlostpointercapture: cancel_pointer, onkeydown: keyboard, onkeyup: key_up, onwheel: wheel,
                    defs {
                        pattern { id: "m1-layout-grid-small", width: "{canvas_grid.spacing_mm}", height: "{canvas_grid.spacing_mm}", pattern_units: "userSpaceOnUse",
                            circle { cx: "0", cy: "0", r: "{canvas_grid.radius_mm}", fill: "var(--wb-grid-large)", stroke: "none" }
                        }
                    }
                    rect { x: "{view_x}", y: "{view_y}", width: "{width}", height: "{height}", fill: "url(#m1-layout-grid-small)" }
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
                            if let Some(active) = matrix_placement.placement.as_ref() {
                                {
                                    let cells = crate::mirrored_pair_geometry::preview_cells(&active.scene, &active.matrix);
                                    rsx! {
                                        g {
                                            class: "m1-mirrored-pair-preview m1-matrix-placement-preview",
                                            "aria-label": "Matrix placement preview",
                                            transform: "translate({active.matrix.origin.x} {active.matrix.origin.y})",
                                            for cell in cells {
                                                g { key: "{cell.row}-{cell.column}", transform: "translate({cell.center.x} {cell.center.y}) rotate({cell.rotation})",
                                                    rect { class: "m1-mirrored-pair-cell", x: "{-cell.size.x / 2.0}", y: "{-cell.size.y / 2.0}", width: "{cell.size.x}", height: "{cell.size.y}", rx: "1" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
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
                                            let target_part_id = cell.member_id.clone();
                                            let runtime = runtime.clone();
                                            let adapter = adapter.clone();
                                            let svg = svg.clone();
                                            let space_down = space_down.clone();
                                            let scope = render_scope.clone();
                                            let generation = render_generation;
                                            let range_ids = visible_ids.clone();
                                            let selection_kind = layout_selection_kind;
                                            let owner = layout_owner.clone();
                                            let tree_cell_anchor = tree_cell_anchor.clone();
                                            let mut pending_splay_pick = pending_splay_origin_pick;
                                            let transform_inspector = matrix_transform_inspector.clone();
                                            let canvas_interaction = canvas_interaction.clone();
                                            let mirrored_pair = mirrored_pair.clone();
                                            rsx! { rect { class: if selected { "m1-matrix-key is-selected" } else { "m1-matrix-key" }, x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}", rx: "0.9", transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation})", "data-matrix-id": "{matrix.id}", "data-row": "{cell.row}", "data-column": "{cell.column}",
                                                onpointerdown: move |event: PointerEvent| {
                                                    let Some(pointer) = event.data().try_as_web_event() else { return; };
                                                    if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair) && mirrored_pair.placement.is_some() { return; }
                                                    if canvas_interaction.current().is_some() { pointer.prevent_default(); pointer.stop_propagation(); return; }
                                                    if pointer.button() != 0 { return; }
                                                    if space_down.get() { return; }
                                                    if pending_splay_pick.peek().is_some() {
                                                        if let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height) {
                                                            pointer.prevent_default();
                                                            pointer.stop_propagation();
                                                            apply_pending_splay_origin_pick(&runtime, workspace, &adapter, &owner, &mut pending_splay_pick, &transform_inspector, point);
                                                            return;
                                                        }
                                                    }
                                                    pointer.prevent_default();
                                                    pointer.stop_propagation();
                                                    let current = runtime.model();
                                                    if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) { return; }
                                                    let Some(hit_context) = objects::context_for_cell(&current, &matrix_id, cell_row, cell_column) else { return; };
                                                    update_tree_cell_anchor(&tree_cell_anchor, &owner, &current, Some(&hit_context));
                                                    let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
                                                    let context = objects::context_for_selection_kind(&current, &hit_context, selection_kind(), retained.as_ref())
                                                        .map(|projection| projection.context)
                                                        .unwrap_or_else(|| hit_context.clone());
                                                    let mode = if pointer.shift_key() { SelectionMode::Range } else if pointer.ctrl_key() || pointer.meta_key() { SelectionMode::Toggle } else { SelectionMode::Replace };
                                                    if let Some(target_part_id) = target_part_id.clone() {
                                                        selection::submit_matrix_cell_selection(&runtime, &adapter, &scope, generation, selection::MatrixCellSelection { matrix_id: matrix_id.clone(), target_part_id, hit_context: &hit_context, context, mode });
                                                    } else {
                                                        selection::submit_canvas_selection(&runtime, &adapter, &scope, generation, context, mode, if mode == SelectionMode::Range { range_ids.as_ref().clone() } else { Vec::new() });
                                                    }
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
                                let mut pending_splay_pick = pending_splay_origin_pick;
                                let transform_inspector = matrix_transform_inspector.clone();
                                rsx! { if layer_visible { g { key: "{part.id}", class: "m1-scene-part", transform: "translate({pose.at.x},{pose.at.y}) rotate({pose.rotation}) {side_transform}", "data-part-id": "{part.id}",
                                    onpointerdown: move |event: PointerEvent| {
                                        let Some(pointer) = event.data().try_as_web_event() else { return; };
                                        if pointer.button() != 0 { return; }
                                        if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair) && mirrored_pair.placement.is_some() { return; }
                                        if canvas_interaction.current().is_some() { pointer.prevent_default(); pointer.stop_propagation(); return; }
                                        if runtime.scope().as_ref() != Some(&render_scope_for_hit) || (adapter.generation)() != generation_for_hit { return; }
                                        if drag.borrow().is_some() || runtime.model().gesture.is_some() { return; }
                                        pointer.prevent_default(); pointer.stop_propagation();
                                        if pending_splay_pick.peek().is_some() {
                                            if let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height) {
                                                apply_pending_splay_origin_pick(&runtime, workspace, &adapter, &owner, &mut pending_splay_pick, &transform_inspector, point);
                                                return;
                                            }
                                        }
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
                                        let context = projection.map(|projection| projection.context).unwrap_or_else(|| hit_context.clone());
                                        if let objects::TreeContext::Key { matrix_id, row, column } = &hit_context
                                            && id == format!("matrix/{matrix_id}/r{row}c{column}")
                                        {
                                            selection::submit_matrix_cell_selection(&runtime, &adapter, &render_scope_for_hit, generation_for_hit, selection::MatrixCellSelection { matrix_id: matrix_id.clone(), target_part_id: id.clone(), hit_context: &hit_context, context: context.clone(), mode });
                                        } else if !current.selected_part_ids.contains(&id)
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
                                        if mode != SelectionMode::Replace { return; }
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
                        if active_workspace == "Layout"
                            && !layout_assembly_3d()
                            && let Some(tool) = layout_transform_tool()
                            && let Some((matrix, projection, context)) = layout_transform_target.clone()
                        {
                            objects::LayoutTransformToolOverlay {
                                runtime: objects::LayoutTransformRuntime(runtime.clone()),
                                svg: objects::LayoutTransformSvg(svg.clone()),
                                arbiter: canvas_interaction.clone(),
                                owner: layout_owner.clone(),
                                selected_context: adapter.selected_context,
                                scope_generation: adapter.generation,
                                workspace,
                                matrix,
                                projection,
                                context,
                                tool,
                                snap_settings: layout_snap_settings.read().clone(),
                                snap_origins: outline_snap_origins.clone(),
                                splay_affect: matrix_splay_affect,
                                view_x,
                                view_y,
                                width,
                                height,
                                on_finish: EventHandler::new(move |_| layout_transform_tool.set(None)),
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
                        if let Some(projection) = outline_inspector.clone().filter(|projection| (projection.drawing_operation)().is_some()) {
                            outline_lifecycle::OutlineDraftCanvasOverlay {
                                key: "{outline_overlay_key.as_deref().unwrap_or_default()}-draft",
                                projection,
                                runtime: outline_lifecycle::OutlineRuntimeHandle::new(runtime.clone()),
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
                    }
                        }
                        CanvasLayers {
                            trigger_id: String::from("m1-layers-trigger"),
                            list_id: String::from("m1-layers-list"),
                            groups: canvas_layers::layout_groups(),
                        }
                    } else if active_workspace == "Export" {
                        export_workspace::ExportWorkspace {
                            zmk_firmware: Some(zmk_firmware_export_panel),
                            workspace,
                            return_workspace,
                            inspect_open,
                            inspector_settings: inspector_panel_settings,
                        }
                    } else if let Some(input) = canvas_input {
                        {workspace_composition::canvas(input)}
                    } else {
                        span {}
                    }
                    if show_empty_board {
                        empty_board_canvas::EmptyBoardCanvas {
                            editable: model.lifecycle == Lifecycle::Ready
                                && model.durability == (Durability::Saved { revision: snapshot.document.revision })
                                && model.gesture.is_none()
                                && model.display_preview.is_none(),
                            on_matrix: on_empty_board_matrix,
                            on_parts: on_browse_parts,
                        }
                    }
                    if active_workspace == "Layout"
                        && let Some(placement) = part_placement.projection.as_ref()
                    {
                        div { class: "m1-canvas-placement-hint", role: "status",
                            {format!("Place {}", parts::placement_label(&placement.pending.definition))}
                            if let Some(layout_name) = placement.owner.layout_id.as_ref()
                                .and_then(|id| document.layouts.iter().find(|layout| &layout.id == id))
                                .map(|layout| layout.name.as_str())
                            {
                                " in {layout_name}"
                            }
                            " · Click or Enter to place · Esc cancels"
                        }
                    }
                    if active_workspace == "Layout" && pending_splay_origin_pick().is_some() {
                        div { class: "m1-canvas-placement-hint", role: "status",
                            "Pick splay origin · Click the canvas · Esc cancels"
                        }
                    }
                }
                if has_inspector {
                    InspectorPanel { compact_open: inspect_open, settings: inspector_panel_settings,
                        if active_workspace != "Layout" && layout_findings_open() {
                            layout_findings::LayoutFindingsInspector {
                                open: true,
                                document: Rc::new(document.as_ref().clone()),
                                findings: snapshot.scene.findings.clone(),
                                source: layout_findings::Source {
                                    workspace: active_workspace,
                                    scope: render_scope.clone(),
                                    token: snapshot.token,
                                    revision: snapshot.document.revision,
                                    generation: render_generation,
                                },
                                on_close: on_close_layout_findings,
                                on_navigate: on_layout_finding,
                            }
                        } else {
                            {workspace_composition::inspector(inspector_input)}
                        }
                        if let Some((reference, assets, editable, owner)) = board_reference_editor {
                            pcb_board_reference::Editor {
                                reference,
                                assets,
                                disabled: !editable,
                                runtime: pcb_board_reference::BoardReferenceRuntimeHandle::new(
                                    runtime.clone(),
                                ),
                                workspace,
                                adapter: pcb_board_reference::BoardReferenceAdapterHandle::new(
                                    adapter.clone(),
                                ),
                                owner,
                            }
                        }
                    }
                }
            }
            footer { class: "m1-editor-footer",
                button { "aria-label": "Undo", onclick: move |_| undo.submit(Event::Undo { operation_id: undo.operation() }), svg { view_box: "0 0 20 20", fill: "none", stroke: "currentColor", stroke_width: "1.5", "aria-hidden": "true", path { d: "M8 6 4 10l4 4M4 10h7a5 5 0 0 1 5 5" } } }
                button { "aria-label": "Redo", onclick: move |_| redo.submit(Event::Redo { operation_id: redo.operation() }), svg { view_box: "0 0 20 20", fill: "none", stroke: "currentColor", stroke_width: "1.5", "aria-hidden": "true", path { d: "m12 6 4 4-4 4m4-4H9a5 5 0 0 0-5 5" } } }
                if matches!(active_workspace, "Layout" | "PCB" | "Keymap" | "Keycaps") && !layout_assembly_3d() {
                    canvas_status_footer::CanvasStatusFooter {
                        snap_settings: layout_snap_settings.read().clone(),
                        active_part_position,
                        snap_menu_available: matches!(active_workspace, "Layout" | "PCB"),
                        open_menu: layout_command_menu,
                        board_available: true,
                        selection_available: footer_selection_available,
                        zoom_percent,
                        findings_count: Some(keycaps_fit::presented_findings(
                            &snapshot.scene.findings,
                            &snapshot.document,
                        ).len()),
                        on_toggle_findings: on_toggle_layout_findings,
                        on_fit_board: on_fit_keymap_board,
                        on_fit_selection: on_fit_keymap_selection,
                        on_zoom_out: on_zoom_keymap_out,
                        on_zoom_in: on_zoom_keymap_in,
                    }
                    if let Some(guide) = model.snap_guide.as_ref() {
                        span { class: "m1-layout-snap-guide", role: "status", "{guide.label}" }
                    }
                } else {
                    span { "{model.camera.zoom * 100.0:.0}%" }
                }
                if active_workspace != "Export"
                    && !(matches!(active_workspace, "Layout" | "PCB" | "Keymap" | "Keycaps")
                        && !layout_assembly_3d())
                {
                    layout_findings::LayoutFindingsFooterButton {
                        count: keycaps_fit::presented_findings(
                            &snapshot.scene.findings,
                            &snapshot.document,
                        ).len(),
                        on_toggle: on_toggle_layout_findings,
                    }
                }
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

fn zoom_center_at(
    (min_x, max_x, min_y, max_y): (f64, f64, f64, f64),
    location: PointerLocation,
    zoom: f64,
) -> Vec2 {
    let base_width = (max_x - min_x).max(50.0);
    let base_height = (max_y - min_y).max(50.0);
    Vec2 {
        x: location.world.x
            - (min_x + max_x - base_width / zoom) * 0.5
            - location.x_fraction * base_width / zoom,
        y: location.world.y - (min_y + max_y + base_height / zoom) * 0.5
            + location.y_fraction * base_height / zoom,
    }
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
