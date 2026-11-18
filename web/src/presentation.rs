//! Presentation drafts and DOM input are separate from the durable session state.
pub(crate) use boardstudio_web_layout::anchor_badge;
pub(crate) use boardstudio_web_layout::board_inspector;
pub(crate) use boardstudio_web_ui_shared::canvas_layers;
pub(crate) use boardstudio_web_ui_shared::canvas_navigation::{
    CanvasNavigationState, use_canvas_navigation_state,
};
mod canvas_status_footer;
pub(crate) use boardstudio_web_case::case_controller;
pub(crate) use boardstudio_web_case::case_display;
pub(crate) use boardstudio_web_case::case_viewer;
mod case_workspace;
mod command_palette;
mod context_summary;
mod editor;
mod empty_board_canvas;
mod export_workspace;
mod header;
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
#[cfg(test)]
mod layout_anchor_badge_tests;
mod layout_bounds;
mod layout_canvas;
mod layout_component_edits;
#[cfg(test)]
mod layout_component_inspector_tests;
#[cfg(test)]
mod layout_field_preview_tests;
#[cfg(test)]
mod layout_layers_tests;
#[cfg(test)]
mod layout_relation_highlight_tests;
#[cfg(test)]
mod layout_remainder_tests;
#[cfg(test)]
mod layout_spacing_ghost_tests;
#[cfg(test)]
mod layout_split_focus_tests;
pub(crate) use boardstudio_web_layout::layout_findings;
pub(crate) use boardstudio_web_layout::layout_findings_state::{
    ReturnTarget as LayoutFindingReturnTarget, layout_finding_is_live,
    layout_finding_return_is_current, source_matches_layout_owner, use_layout_findings_state,
};
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
pub(crate) use boardstudio_web_ui_model::state::{
    AppearanceState, PreferenceStorageWarning, ThemeState,
};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::{
    CaseGenerationState, CompactPanelState, Drag, FieldPreview, FieldPreviewSource,
    FieldPreviewState, InstanceSelection, LayerVisibility, LayoutOwnerIdentity, RelationHighlight,
    ResolvedAppearance, WorkspaceState,
};
pub(crate) use boardstudio_web_ui_model::svg_coordinates::{
    PointerLocation, coordinates, coordinates_at, coordinates_with_scale, pointer_location,
};
pub(crate) use boardstudio_web_ui_model::{
    canvas_interaction, instance_selection, selection, spacing_ghost,
};
pub(crate) use boardstudio_web_ui_shared::panels::browse_parts_workspace;
mod layout_key_selection_actions;
mod layout_matrix_relations;
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
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonVariant},
    field::FieldError,
};
use part_placement::{LayoutPlacementCancellation, layout_view_mode_handler};
mod parts_workspace;
mod setup_project_name;
mod setup_workspace;
pub(crate) use boardstudio_web_pcb::pcb_layers;
pub(crate) use boardstudio_web_pcb::pcb_module_footprints;
pub(crate) use boardstudio_web_pcb::pcb_module_inspector;
pub(crate) use boardstudio_web_pcb::pcb_physical_setup;
pub(crate) use boardstudio_web_pcb::pcb_scene;
pub(crate) use boardstudio_web_pcb::pcb_wiring;
mod owner_checks;
mod pcb_workspace;
mod pending_picks;
pub(crate) use boardstudio_web_case::shared_viewer;
pub(crate) use boardstudio_web_layout::setup_guide;
mod finding_navigation;
mod notifications;
mod tree_cell_anchor;
mod workbench_shortcuts;
mod workspace_composition;
mod workspace_navigation;
mod zmk_firmware_export;

use crate::case_generation_lifecycle::AutomaticCaseGeneration;
pub(crate) use boardstudio_web_ui_shared::footprint_graphics;
use canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner};
use canvas_layers::CanvasLayers;
use editor::Editor;
pub(crate) use finding_navigation::{
    LayoutFindingNavigationContext, current_keycaps_navigation_owner, findings_owner_is_current,
    fit_selected_bridge, layout_finding_context, layout_findings_for_display,
    perform_layout_finding_navigation,
};
pub(crate) use layout_bounds::{
    accepted_board_contours, keycaps_bounds, keymap_bounds, keymap_toolbar_height, pcb_bounds,
};
use library::Library;
pub(crate) use mechanical_settings::{MechanicalSettings, MechanicalSettingsProps};
pub(crate) use mechanical_settings_mount::MechanicalSettingsMount;
pub(crate) use owner_checks::{
    canvas_owner_is_current, case_setup_context_is_current, current_case_layer_ids,
    pcb_add_layout_open_handler, pcb_add_layout_owner_is_current, pcb_add_outline_select_handler,
    pcb_owner_is_current,
};
use panels::{
    DockLayout, InspectorPanel, ObjectsPanel, PanelMode, PanelSettings, PanelSide,
    use_panel_settings,
};
use parts::{PartsQuery, PartsSelection};
pub(crate) use pending_picks::{
    PendingAnchorPick, PendingSplayOriginPick, apply_pending_anchor_pick,
    apply_pending_splay_origin_pick,
};
use selection::{ReentrancyReset, SelectionAdapter};
use setup_guide::{PendingNewKeyboard, SetupGuideRequest, SetupGuideStage};
pub(crate) use setup_project_name::{SetupProjectNameEdit, use_setup_project_name};
pub(crate) use tree_cell_anchor::{
    OwnedTreeCellAnchor, tree_cell_anchor_for_owner, update_tree_cell_anchor,
};
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
    CompanionPlacementTarget, Contour, EditOperation, Finding as LayoutFinding, HalfExceptionState,
    MirroredExceptionState, MirroredItemKind, Part, PartKind, PartOutline, Position, Vec2,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use footprint_graphics::FootprintGraphics;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashSet},
    rc::Rc,
};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{HtmlElement, SvgElement};

#[derive(Clone, Copy)]
pub(super) struct ExportReturnWorkspace(pub(super) Signal<&'static str>);

#[derive(Clone, Copy)]
struct WorkspaceRoutingSlots {
    select_tree: EventHandler<objects::TreeSelectRequest>,
    navigate: EventHandler<(Scope, String, Option<String>)>,
}

#[allow(non_snake_case)]
pub fn App() -> Element {
    rsx! { notifications::NotificationProvider { WorkbenchApp {} } }
}

#[component]
fn WorkbenchApp() -> Element {
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
    let project_browser = use_signal(|| false);
    use_context_provider(|| library::ProjectBrowserState(project_browser));
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
    let appearance = use_signal(move || panels::read_appearance_preference(preference_warning));
    let system_appearance = use_signal(read_system_appearance);
    use_context_provider(|| AppearanceState(appearance));
    let theme = use_signal(move || panels::read_workbench_theme_preference(preference_warning));
    use_context_provider(|| ThemeState(theme));
    let resolved_appearance = use_memo(move || {
        let preference = appearance();
        if preference == "system" {
            system_appearance()
        } else {
            preference
        }
    });
    use_context_provider(|| ResolvedAppearance(resolved_appearance));
    let media_listener = use_hook(|| {
        Rc::new(RefCell::new(
            None::<(web_sys::MediaQueryList, Closure<dyn FnMut(web_sys::Event)>)>,
        ))
    });
    use_effect({
        let mut system_appearance = system_appearance;
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
                    system_appearance.set(if observed_query.matches() {
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
    use_effect(use_reactive((&resolved_appearance(), &theme()), {
        move |(effective, theme)| {
            if let Some(root) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.document_element())
            {
                let _ = root.set_attribute("data-appearance", effective);
                let _ = root.set_attribute("data-theme", theme);
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
    let mut settings_open = use_signal(|| false);
    let palette = command_palette::use_palette_state();
    boardstudio_web_ui_shared::ui::command::use_command_registry();
    library::use_project_tools();
    let header_undo = runtime.clone();
    let header_redo = runtime.clone();
    let on_undo = EventHandler::new(move |()| {
        header_undo.submit(Event::Undo {
            operation_id: header_undo.operation(),
        })
    });
    let on_redo = EventHandler::new(move |()| {
        header_redo.submit(Event::Redo {
            operation_id: header_redo.operation(),
        })
    });
    let on_export = EventHandler::new(move |()| {
        if workspace() == "Export" {
            workspace.set(return_workspace())
        } else {
            workspace.set("Export")
        }
    });
    command_palette::use_navigation_commands(
        workspace,
        project_browser,
        runtime.model().accepted.is_some(),
        on_undo,
        on_redo,
        on_export,
    );
    rsx! {
        link { rel: "stylesheet", href: "assets/workbench-base.css" }
        link { rel: "stylesheet", href: "assets/header.css" }
        link { rel: "stylesheet", href: "assets/command-palette.css" }
        link { rel: "stylesheet", href: "assets/shell.css" }
        link { rel: "stylesheet", href: "assets/firmware-keymap-panel.css" }
        link { rel: "stylesheet", href: "assets/tailwind.css" }
        main { class: "flex h-dvh min-h-0 w-full flex-col overflow-hidden",
            onkeydown: {
                let runtime = runtime.clone();
                move |event: KeyboardEvent| {
                    if !workbench_shortcuts::handle_palette_shortcut(palette, &event) {
                        workbench_shortcuts::handle_history_shortcut(runtime.clone(), event);
                    }
                }
            },
            header::Header {
                project_menu: rsx! { library::ProjectHeaderMenu { project_name, on_undo, on_redo, on_export,
                    on_settings: move |_| settings_open.set(true),
                } },
                workspace,
                show_workspaces: runtime.model().accepted.is_some() && !project_browser(),
                has_project: runtime.model().accepted.is_some(),
                save_state,
                settings_open,
                palette_open: palette.open,
                on_search: move |_| palette.show(),
                on_undo,
                on_redo,
                on_export,
                panel_controls: rsx! {
                    if runtime.model().accepted.is_some() && workspace() != "Export" { panels::CompactPanelControls {} }
                },
            }
            command_palette::CommandPalette { state: palette }
            if runtime.model().accepted.is_some() {
                div { class: "flex min-h-0 flex-1 flex-col", hidden: project_browser(), Editor {} }
            }
            if project_browser() || runtime.model().accepted.is_none() { LibraryLanding {} }
                notifications::RuntimeNotifications {}
            if preference_warning() {
                p { role: "status", "aria-live": "polite", class: "m-0 flex-none truncate border-t border-border bg-card px-3 py-[5px] text-xs text-muted-foreground",
                    "Preferences are session-only because browser storage is unavailable."
                }
            }
            if !new_keyboard_error().is_empty() {
                p { role: "alert", class: "m-0 flex-none truncate border-t border-border bg-card px-3 py-[5px] text-xs text-muted-foreground", "{new_keyboard_error()}" }
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
        if !runtime.status_is_transient() && (alert || status != "Saved locally.") {
            p {
                role: if alert { "alert" } else { "status" },
                "aria-live": if alert { "assertive" } else { "polite" },
                class: "m-0 flex-none truncate border-t border-border bg-card px-3 py-[5px] text-xs text-muted-foreground",
                "{status}"
            }
        }
    }
}

fn read_system_appearance() -> &'static str {
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
fn LibraryLanding() -> Element {
    use boardstudio_web_ui_shared::ui::card::Card;
    rsx! {
        Card { class: "block min-h-0 w-full flex-1 overflow-hidden border-0 bg-canvas p-0 [&>[id=m1-library-landing-content]]:grid [&>[id=m1-library-landing-content]]:h-full [&>[id=m1-library-landing-content]]:min-h-0 [&>[id=m1-library-landing-content]]:w-full [&>[id=m1-library-landing-content]]:max-w-none [&>[id=m1-library-landing-content]]:grid-cols-[minmax(0,2.7fr)_minmax(280px,1fr)] [&>[id=m1-library-landing-content]]:gap-0 max-[1100px]:[&>[id=m1-library-landing-content]]:grid-cols-[minmax(0,1.8fr)_minmax(260px,1fr)] max-[760px]:overflow-auto max-[760px]:[&>[id=m1-library-landing-content]]:flex max-[760px]:[&>[id=m1-library-landing-content]]:h-auto max-[760px]:[&>[id=m1-library-landing-content]]:min-h-full max-[760px]:[&>[id=m1-library-landing-content]]:flex-col",
            Library { project_menu: false }
        }
    }
}

#[cfg(test)]
#[component]
fn ExportPanel(#[props(default)] zmk_firmware: Option<ZmkFirmwareExportPanelInput>) -> Element {
    use boardstudio_web_ui_shared::ui::{
        checkbox::{Checkbox, CheckboxState},
        label::Label,
    };
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let model = runtime.model();
    let archive = runtime.clone();
    let preference = runtime.clone();
    let step = runtime.clone();
    rsx! {
        section { class: "m-auto w-[min(100%,760px)] p-7 text-foreground [&>p]:max-w-[60ch] [&>p]:text-muted-foreground [&>h2]:mt-6 [&>h2]:mb-2.5", "aria-label": "Export",
            h1 { "Export" }
            p { "Create files from the saved keyboard in the current board and instance scope." }
            div { class: "my-5 flex flex-wrap gap-2",
                Button { variant: ButtonVariant::Secondary, disabled: model.accepted.is_none(), onclick: move |_| step.export_step(), "Export STEP" }
            }
            if let Some(firmware) = zmk_firmware {
                h2 { "Design files" }
                div { class: "grid gap-2",
                    ZmkFirmwareExportRow { ready: firmware.ready, on_export: firmware.on_export }
                }
            }
            section { class: "mt-6 grid gap-2.5 rounded-md border border-border bg-card p-4 [&_h2]:m-0 [&_p]:m-0 [&_p]:text-muted-foreground", "aria-label": "Portable project",
                h2 { "Portable project" }
                p { "Keep an editable copy of the whole project, including all boards." }
                Label { class: "flex items-start gap-2.5 [&_span]:grid [&_span]:gap-[3px] [&_small]:text-xs [&_small]:text-muted-foreground",
                    Checkbox {
                        aria_label: "Embed used models",
                        checked: if runtime.embed_used_models() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                        on_checked_change: move |state: CheckboxState| preference.set_embed_used_models(state == CheckboxState::Checked),
                    }
                    span { strong { "Embed used models" } small { "Include attached 3D model files used in this project." } }
                }
                Button { variant: ButtonVariant::Secondary,
                    class: "justify-self-start",
                    disabled: model.accepted.is_none(),
                    onclick: move |_| archive.export_project_copy(),
                    "Save .boardstudio project"
                }
            }
            RuntimeReportBanner {}
        }
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
        matrix_id,
        assembly_id,
        ..
    } = &selected.context
    else {
        return None;
    };
    if model.selected_part_ids.is_empty()
        || model.selected_part_ids.first() != Some(part_id)
        || !selection::context_is_current(model, &selected.scope, &selected.context)
    {
        return None;
    }
    let snapshot = model.accepted.as_ref()?;
    let document = &snapshot.document;
    let part = document.parts.iter().find(|part| part.id == *part_id)?;
    let mirrored_half = mirrored_half_projection(document, part_id);
    if matrix_id.is_some() && assembly_id.is_some() && mirrored_half.is_none() {
        return None;
    }
    let definition = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)?;
    let board = document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)?;
    let board_part_ids: BTreeSet<_> = board.part_ids.iter().map(String::as_str).collect();
    let parts_by_id: BTreeMap<_, _> = document
        .parts
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();
    if model.selected_part_ids.iter().any(|selected_id| {
        !board_part_ids.contains(selected_id.as_str())
            || !parts_by_id.contains_key(selected_id.as_str())
    }) {
        return None;
    }
    let mut matrix_key_ids = BTreeSet::new();
    for matrix in &document.matrices {
        let Some(matrix_scene) = snapshot
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix.id)
        else {
            continue;
        };
        let matrix_part_ids: BTreeSet<_> = matrix.part_ids.iter().map(String::as_str).collect();
        matrix_key_ids.extend(
            matrix_scene
                .cells
                .iter()
                .filter(|cell| cell.enabled)
                .filter_map(|cell| cell.member_id.as_deref())
                .filter(|id| matrix_part_ids.contains(id)),
        );
    }
    let board_parts: Vec<_> = board
        .part_ids
        .iter()
        .filter_map(|id| {
            parts_by_id
                .get(id.as_str())
                .map(|part| inspector::BoardPartChoice {
                    id: part.id.clone(),
                    reference: part.reference.clone(),
                    matrix_key: matrix_key_ids.contains(part.id.as_str()),
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
                    layout.matrix_ids.contains(&matrix.id) && matrix.part_ids.contains(part_id)
                }))
    });
    let mut active_constraint = None;
    for selected_id in &model.selected_part_ids {
        match document.constraint_driving_part(selected_id) {
            Ok(Some(constraint)) => {
                active_constraint = Some(constraint.clone());
                break;
            }
            Ok(None) => {}
            Err(_) => return None,
        }
    }
    let mut relationship_summary =
        layout_relationship_summary(document, board, active_layout, Some(part));
    // Ticket 13: the anchor fact leads for anchored parts.
    if let Some(lead) = part_anchor_summary(document, part) {
        relationship_summary = format!("{lead} {relationship_summary}");
    }
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
        // Ticket 13: the Inspector edits board-space positions, so an anchored
        // part shows its evaluated pose (what the canvas shows); a detached
        // part keeps its stored half-frame pose.
        position: if part.anchor.is_some() {
            boardstudio_core::evaluated_part_pose(document, part).at
        } else {
            part.pose.at
        },
        selection_count: model.selected_part_ids.len(),
        layout_id: active_layout.map(|layout| layout.id.clone()),
        layouts,
        outline: part.outline.clone().unwrap_or_else(PartOutline::default),
        board_parts,
        active_constraint,
        relationship_summary,
        mirrored_half,
        part_mirror: part_mirror_projection(document, part_id, active_layout, is_matrix_member),
        anchor: part_anchor_projection(document, part),
        // Ticket 13: matrix members and companions follow the matrix refresh,
        // so they get no anchor vocabulary and no Pick form.
        anchor_choices: if is_matrix_member {
            Vec::new()
        } else {
            part_anchor_choices(document, active_layout)
        },
    })
}

/// Ticket 13: the part's column-vocabulary anchor for the Inspector: the
/// anchor key and Follow subset, the carried Spread/Stagger/Splay and the
/// local Offset/Rotation. `None` is a detached part; parts homed on a
/// layout without an anchor stay detached.
fn part_anchor_projection(
    document: &boardstudio_core::model::ProjectDoc,
    part: &boardstudio_core::model::Part,
) -> Option<inspector::PartAnchorProjection> {
    let anchor = part.anchor.as_ref()?;
    let matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == anchor.matrix_id)?;
    Some(inspector::PartAnchorProjection {
        matrix_id: anchor.matrix_id.clone(),
        matrix_name: matrix.name.clone().unwrap_or(matrix.id.clone()),
        row: anchor.row,
        column: anchor.column,
        follow: anchor.follow,
        spread: anchor.spread,
        stagger: anchor.stagger,
        splay: anchor.splay,
        offset: anchor.offset,
        rotation: anchor.rotation,
    })
}

/// Ticket 13: the anchor-key choices for Pick: every matrix on the part's
/// layout with its live dimensions.
fn part_anchor_choices(
    document: &boardstudio_core::model::ProjectDoc,
    active_layout: Option<&boardstudio_core::model::Layout>,
) -> Vec<inspector::AnchorKeyChoice> {
    let Some(layout) = active_layout else {
        return Vec::new();
    };
    layout
        .matrix_ids
        .iter()
        .filter_map(|matrix_id| {
            document
                .matrices
                .iter()
                .find(|matrix| matrix.id == *matrix_id)
                .map(|matrix| inspector::AnchorKeyChoice {
                    matrix_id: matrix.id.clone(),
                    matrix_name: matrix.name.clone().unwrap_or(matrix.id.clone()),
                    rows: matrix.rows,
                    columns: matrix.columns,
                })
        })
        .collect()
}

/// Ticket 12: the "Mirror on both halves" toggle for an independent part on a
/// stored-gap half. `Some(true)` while a live Part correspondence mirrors it;
/// `Some(false)` while it stays local. Members keep the exception buttons.
fn part_mirror_projection(
    document: &boardstudio_core::model::ProjectDoc,
    part_id: &str,
    active_layout: Option<&boardstudio_core::model::Layout>,
    is_matrix_member: bool,
) -> Option<bool> {
    if is_matrix_member {
        return None;
    }
    let layout = active_layout?;
    boardstudio_core::layout_half_offset(document, &layout.id)?;
    // Ticket 12: Part correspondences live on the layout mirror links, next
    // to the matrices they mirror — not on the hardware halves. The toggle
    // reads mirrored only while both ends of the record still exist.
    let mirrored =
        boardstudio_core::part_correspondence(document, part_id).is_some_and(|(_, record)| {
            document
                .parts
                .iter()
                .any(|part| part.id == record.source_id)
                && document
                    .parts
                    .iter()
                    .any(|part| part.id == record.mirror_id)
        });
    Some(mirrored)
}

fn mirrored_half_projection(
    document: &boardstudio_core::model::ProjectDoc,
    part_id: &str,
) -> Option<inspector::MirroredHalfProjection> {
    let hardware = document.hardware.as_ref()?;
    for pair in &hardware.pairs {
        let Some(halves) = pair.halves.as_ref() else {
            continue;
        };
        let Some(link) = halves.correspondences.iter().find(|link| {
            link.kind == MirroredItemKind::Part
                && (link.source_id == part_id || link.mirror_id == part_id)
        }) else {
            continue;
        };
        let state = halves
            .exceptions
            .iter()
            .find(|exception| {
                exception.kind == MirroredItemKind::Part && exception.mirror_id == link.mirror_id
            })
            .map(|exception| match exception.state {
                MirroredExceptionState::Local => HalfExceptionState::Local,
                MirroredExceptionState::Omitted => HalfExceptionState::Omitted,
            })
            .unwrap_or(HalfExceptionState::Linked);
        return Some(inspector::MirroredHalfProjection {
            pair_id: pair.id.clone(),
            mirror_id: link.mirror_id.clone(),
            state,
        });
    }
    None
}

fn companion_placement_projection(
    model: &ReadModel,
    selected: Option<&objects::ScopedTreeContext>,
    context_generation: u64,
    scope_generation: u64,
) -> Option<inspector::CompanionPlacementProjection> {
    let selected = selected?;
    if !selection::context_is_current(model, &selected.scope, &selected.context) {
        return None;
    }
    let snapshot = model.accepted.as_ref()?;
    let document = &snapshot.document;
    let hardware = document.hardware.as_ref()?;
    let component_context = matches!(&selected.context, objects::TreeContext::Component { .. });
    let (matrix_id, row, column) = match &selected.context {
        objects::TreeContext::Matrix { matrix_id } => (Some(matrix_id.as_str()), None, None),
        objects::TreeContext::Row { matrix_id, row } => {
            (Some(matrix_id.as_str()), Some(*row), None)
        }
        objects::TreeContext::Column { matrix_id, column } => {
            (Some(matrix_id.as_str()), None, Some(*column))
        }
        objects::TreeContext::Key {
            matrix_id,
            row,
            column,
        } => (Some(matrix_id.as_str()), Some(*row), Some(*column)),
        objects::TreeContext::Component {
            matrix_id,
            row,
            column,
            ..
        } => (matrix_id.as_deref(), *row, *column),
        _ => return None,
    };

    let mut matches = Vec::new();
    for pair in &hardware.pairs {
        let Some(halves) = pair.halves.as_ref() else {
            continue;
        };
        if let Some(matrix_id) = matrix_id.filter(|_| !component_context) {
            let linked_matrix = halves.correspondences.iter().any(|link| {
                link.kind == MirroredItemKind::Matrix
                    && (link.source_id == matrix_id || link.mirror_id == matrix_id)
            });
            if !linked_matrix {
                continue;
            }
            let (target, key) = match (row, column) {
                (Some(row), Some(column)) => (None, Some((row, column))),
                (Some(row), None) => (
                    Some(CompanionPlacementTarget::Row {
                        matrix_id: matrix_id.to_owned(),
                        row,
                    }),
                    None,
                ),
                (None, Some(column)) => (
                    Some(CompanionPlacementTarget::Column {
                        matrix_id: matrix_id.to_owned(),
                        column,
                    }),
                    None,
                ),
                (None, None) => (
                    Some(CompanionPlacementTarget::Matrix {
                        matrix_id: matrix_id.to_owned(),
                    }),
                    None,
                ),
            };
            let (eligible, disabled_reason) =
                companion_links_for_selection(document, pair, matrix_id, row, column);
            if eligible.is_empty() && disabled_reason.is_none() {
                continue;
            }
            let targets = if key.is_some() {
                eligible
                    .iter()
                    .map(|link| CompanionPlacementTarget::Item {
                        item_kind: MirroredItemKind::Part,
                        mirror_id: link.mirror_id.clone(),
                    })
                    .collect()
            } else {
                target.into_iter().collect()
            };
            matches.push((pair, targets, eligible, disabled_reason));
        } else {
            let selected_parts = &model.selected_part_ids;
            let mut eligible = Vec::new();
            let mut targets = Vec::new();
            let mut disabled_reason = None;
            for link in halves
                .correspondences
                .iter()
                .filter(|link| link.kind == MirroredItemKind::Part)
            {
                if !selected_parts
                    .iter()
                    .any(|part_id| part_id == &link.source_id || part_id == &link.mirror_id)
                {
                    continue;
                }
                if let Some(reason) = boardstudio_core::companion_placement_disabled_reason(
                    document,
                    &pair.id,
                    &link.source_id,
                ) {
                    disabled_reason = Some(reason);
                    continue;
                }
                if !is_matrix_companion(document, pair, &link.source_id) {
                    continue;
                }
                if !eligible.iter().any(
                    |existing: &&boardstudio_core::model::MirroredCorrespondence| {
                        existing.mirror_id == link.mirror_id
                    },
                ) {
                    eligible.push(link);
                    targets.push(CompanionPlacementTarget::Item {
                        item_kind: MirroredItemKind::Part,
                        mirror_id: link.mirror_id.clone(),
                    });
                }
            }
            if let (Some(matrix_id), Some(row), Some(column)) = (matrix_id, row, column)
                && targets.is_empty()
            {
                let (cell_links, cell_reason) = companion_links_for_selection(
                    document,
                    pair,
                    matrix_id,
                    Some(row),
                    Some(column),
                );
                eligible = cell_links;
                targets.extend(eligible.iter().map(|link| CompanionPlacementTarget::Item {
                    item_kind: MirroredItemKind::Part,
                    mirror_id: link.mirror_id.clone(),
                }));
                disabled_reason = disabled_reason.or(cell_reason);
            }
            if !targets.is_empty() || disabled_reason.is_some() {
                matches.push((pair, targets, eligible, disabled_reason));
            }
        }
    }
    let [(pair, targets, eligible, disabled_reason)] = matches.as_slice() else {
        return None;
    };
    let halves = pair.halves.as_ref()?;
    let modes: Vec<_> = if eligible.is_empty() {
        vec![halves.companion_mode]
    } else {
        eligible
            .iter()
            .map(|link| link.companion_mode.unwrap_or(halves.companion_mode))
            .collect()
    };
    let first_mode = modes.first().copied();
    let mixed_mode = modes.iter().any(|mode| Some(*mode) != first_mode);
    let tweaks: Vec<_> = eligible.iter().map(|link| link.companion_tweak).collect();
    let first_tweak = tweaks.first().copied().unwrap_or(None);
    let mixed_tweak = tweaks.iter().any(|tweak| *tweak != first_tweak);
    let snapshot_token = snapshot.token;
    Some(inspector::CompanionPlacementProjection {
        owner: inspector::CompanionPlacementOwner {
            scope: selected.scope.clone(),
            context: selected.clone(),
            selected_part_ids: model.selected_part_ids.clone(),
            snapshot_token,
            revision: document.revision,
            context_generation,
            scope_generation,
        },
        pair_id: pair.id.clone(),
        targets: targets.clone(),
        mode: if mixed_mode { None } else { first_mode },
        mixed_mode,
        tweak: if mixed_tweak { None } else { first_tweak },
        mixed_tweak,
        disabled_reason: if eligible.is_empty() {
            *disabled_reason
        } else {
            None
        },
    })
}

fn companion_links_for_selection<'a>(
    document: &'a boardstudio_core::model::ProjectDoc,
    pair: &'a boardstudio_core::model::MechanicalMirrorPair,
    selected_matrix_id: &str,
    row: Option<u32>,
    column: Option<u32>,
) -> (
    Vec<&'a boardstudio_core::model::MirroredCorrespondence>,
    Option<&'static str>,
) {
    let Some(halves) = pair.halves.as_ref() else {
        return (Vec::new(), None);
    };
    let matrix_links: Vec<_> = halves
        .correspondences
        .iter()
        .filter(|link| {
            link.kind == MirroredItemKind::Matrix
                && (link.source_id == selected_matrix_id || link.mirror_id == selected_matrix_id)
        })
        .collect();
    let [matrix_link] = matrix_links.as_slice() else {
        return (Vec::new(), None);
    };
    let Some(matrix) = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_link.source_id)
    else {
        return (Vec::new(), None);
    };
    let mut links = Vec::new();
    let mut disabled_reason = None;
    for cell in matrix.cells.iter().filter(|cell| {
        cell.enabled
            && !cell.deleted
            && row.is_none_or(|row| cell.row == row)
            && column.is_none_or(|column| cell.column == column)
    }) {
        let prefix = format!("matrix/{}/r{}c{}/", matrix.id, cell.row, cell.column);
        for assembly in &cell.assemblies {
            let source_id = format!("{prefix}{}", assembly.id);
            let Some(link) = halves
                .correspondences
                .iter()
                .find(|link| link.kind == MirroredItemKind::Part && link.source_id == source_id)
            else {
                continue;
            };
            if let Some(reason) = boardstudio_core::companion_placement_disabled_reason(
                document, &pair.id, &source_id,
            ) {
                disabled_reason = Some(reason);
            } else if !links.iter().any(
                |existing: &&boardstudio_core::model::MirroredCorrespondence| {
                    existing.mirror_id == link.mirror_id
                },
            ) {
                links.push(link);
            }
        }
    }
    (links, disabled_reason)
}

fn is_matrix_companion(
    document: &boardstudio_core::model::ProjectDoc,
    pair: &boardstudio_core::model::MechanicalMirrorPair,
    source_id: &str,
) -> bool {
    companion_links_for_selection(
        document,
        pair,
        &source_matrix_id(pair, source_id).unwrap_or_default(),
        None,
        None,
    )
    .0
    .iter()
    .any(|link| link.source_id == source_id)
}

fn source_matrix_id(
    pair: &boardstudio_core::model::MechanicalMirrorPair,
    source_part_id: &str,
) -> Option<String> {
    let halves = pair.halves.as_ref()?;
    halves
        .correspondences
        .iter()
        .find(|matrix| {
            matrix.kind == MirroredItemKind::Matrix
                && source_part_id.starts_with(&format!("matrix/{}/", matrix.source_id))
        })
        .map(|matrix| matrix.source_id.clone())
}

/// Resolve a Relations reference against the live accepted document. A component opens
/// its Inspector context, as for a finding. A target that has gone, or a request for a
/// session that has moved on, resolves to `None` and nothing is selected.
fn relation_target_request(
    runtime: &Runtime,
    mut request: objects::TreeSelectRequest,
) -> Option<objects::TreeSelectRequest> {
    if runtime.scope().as_ref() != Some(&request.scope) {
        return None;
    }
    let model = runtime.model();
    if let objects::TreeContext::Component {
        part_id: Some(part_id),
        ..
    } = &request.context
    {
        request.context = objects::component_context_for_finding_part(&model, part_id)?;
    }
    if !selection::context_is_current(&model, &request.scope, &request.context) {
        return None;
    }
    // A removed key keeps a resolvable (empty) context, so check the cell itself.
    if let objects::TreeContext::Key {
        matrix_id,
        row,
        column,
    } = &request.context
    {
        let document = &model.accepted.as_ref()?.document;
        let removed = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == *matrix_id)?
            .cells
            .iter()
            .any(|cell| cell.row == *row && cell.column == *column && cell.deleted);
        if removed {
            return None;
        }
    }
    Some(request)
}

/// The Relations projection for the selected matrix context, from the accepted document and
/// Core's matrix scene. `None` when nothing is selected, the selection is stale, or the
/// context names no live object of a matrix.
fn matrix_context_relations(
    model: &ReadModel,
    selected: Option<&objects::ScopedTreeContext>,
) -> Option<boardstudio_web_layout::matrix_relations::MatrixRelations> {
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
    let scene = snapshot
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == *matrix_id)?;
    let mut relations = boardstudio_web_layout::matrix_relations::matrix_relations(
        &snapshot.document,
        scene,
        &selected.context,
        &snapshot.scene.matrix_scenes,
    )?;
    // The chain covers one matrix context only: selected parts in other
    // matrices get an explanatory note, never a merged frame.
    relations.selection_note = boardstudio_web_layout::matrix_relations::cross_matrix_note(
        &snapshot.scene.matrix_scenes,
        &selected.context,
        &model.selected_part_ids,
    );
    Some(relations)
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
    let driven =
        active_part.and_then(
            |target| match document.constraint_driving_part(&target.id) {
                Ok(Some(constraint)) => {
                    let source = document
                        .parts
                        .iter()
                        .find(|part| part.id == constraint.source())
                        .map(|part| part.reference.as_str())
                        .unwrap_or(constraint.source());
                    let driven = document
                        .parts
                        .iter()
                        .find(|part| part.id == constraint.target())
                        .map(|part| part.reference.as_str())
                        .unwrap_or(constraint.target());
                    Some(format!("{source} drives {driven}."))
                }
                Ok(None) => None,
                Err(_) => {
                    Some("The saved placement relationship could not be verified.".to_owned())
                }
            },
        );
    driven.unwrap_or_else(|| "No saved placement relationship on this selection.".to_owned())
}

/// Ticket 13: the Relationships lead for an anchored part: its anchor key and
/// Follow subset, ahead of the mirror-pairing summary.
fn part_anchor_summary(
    document: &boardstudio_core::model::ProjectDoc,
    part: &boardstudio_core::model::Part,
) -> Option<String> {
    let anchor = part.anchor.as_ref()?;
    let matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == anchor.matrix_id)?;
    let name = matrix.name.clone().unwrap_or(matrix.id.clone());
    let mut follows = Vec::new();
    if anchor.follow.x {
        follows.push("X");
    }
    if anchor.follow.y {
        follows.push("Y");
    }
    if anchor.follow.rotation {
        follows.push("rotation");
    }
    let follows = if follows.is_empty() {
        "none".to_owned()
    } else {
        follows.join(", ")
    };
    Some(format!(
        "Anchored to {} R{}C{}, following {}.",
        name,
        anchor.row + 1,
        anchor.column + 1,
        follows
    ))
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
        matrix_id,
        assembly_id,
        ..
    } = &selected.context
    else {
        return None;
    };
    let snapshot = model.accepted.as_ref()?;
    if matrix_id.is_some()
        && assembly_id.is_some()
        && mirrored_half_projection(&snapshot.document, part_id).is_none()
    {
        return None;
    }
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
                ..
            } if part_id == &owner.part_id
        )
        && selection::context_is_current(&model, &owner.scope, &selected.context)
        && model.selected_part_ids == owner.selected_part_ids
        && model.selected_part_ids.first() == Some(&owner.part_id)
}

fn companion_placement_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &inspector::CompanionPlacementOwner,
    pair_id: &str,
    submitted_targets: &[CompanionPlacementTarget],
) -> bool {
    let current_owner = current_layout_owner(runtime, workspace, adapter);
    if current_owner.workspace != "Layout"
        || current_owner.scope.as_ref() != Some(&owner.scope)
        || current_owner.token != Some(owner.snapshot_token)
        || current_owner.revision != Some(owner.revision)
        || owner.context_generation != (adapter.generation)()
        || owner.scope_generation != (adapter.generation)()
    {
        return false;
    }
    let model = runtime.model();
    let Some(selected) = adapter.selected_context.read().clone() else {
        return false;
    };
    if selected != owner.context
        || model.selected_part_ids != owner.selected_part_ids
        || !selection::context_is_current(&model, &owner.scope, &owner.context.context)
    {
        return false;
    }
    let Some(projection) = companion_placement_projection(
        &model,
        Some(&selected),
        owner.context_generation,
        owner.scope_generation,
    ) else {
        return false;
    };
    if projection.pair_id != pair_id {
        return false;
    }
    let mut expected = projection.targets;
    if submitted_targets
        .iter()
        .any(|target| matches!(target, CompanionPlacementTarget::Pair))
    {
        expected.push(CompanionPlacementTarget::Pair);
    }
    sort_companion_targets(&mut expected);
    let mut submitted = submitted_targets.to_vec();
    sort_companion_targets(&mut submitted);
    expected == submitted
}

fn sort_companion_targets(targets: &mut Vec<CompanionPlacementTarget>) {
    targets.sort_by_key(|target| serde_json::to_string(target).unwrap_or_default());
    targets.dedup();
}

/// Begin the pending edit for an Inspector field and park it on its field of the
/// Inspector's pending-edit state, so the form renders the draft while it is pending and
/// the accepted value again when it settles.
fn submit_layout_component_edit(
    runtime: &Rc<Runtime>,
    pending_edits: Signal<layout_component_edits::LayoutComponentInspectorEdits>,
    field: layout_component_edits::InspectorField,
    resolver: boardstudio_application::EditResolver,
    draft: Option<String>,
) {
    let helper = pending_edits.peek().clone();
    let label = match field {
        layout_component_edits::InspectorField::X => "layout-inspector-x",
        layout_component_edits::InspectorField::Y => "layout-inspector-y",
        layout_component_edits::InspectorField::Margin => "layout-inspector-outline",
        layout_component_edits::InspectorField::Layout => "layout-inspector-layout",
        layout_component_edits::InspectorField::Constraint => "layout-inspector-constraint",
        layout_component_edits::InspectorField::RemoveConstraint => {
            "layout-inspector-remove-constraint"
        }
        layout_component_edits::InspectorField::MirroredHalfState => "layout-mirrored-half-state",
        layout_component_edits::InspectorField::PartMirror => "layout-part-mirror",
        layout_component_edits::InspectorField::CompanionPlacement => "layout-companion-placement",
        layout_component_edits::InspectorField::PartAnchorValue => "layout-part-anchor-value",
        layout_component_edits::InspectorField::PartAnchorFollow => "layout-part-anchor-follow",
        layout_component_edits::InspectorField::PartAnchorPick => "layout-part-anchor-pick",
        layout_component_edits::InspectorField::PartAnchorDetach => "layout-part-anchor-detach",
    };
    if field.is_field() {
        helper.begin_field(
            runtime,
            field,
            label,
            Some("layout".into()),
            resolver,
            draft.as_deref().unwrap_or_default(),
        );
    } else {
        helper.begin_one_shot(runtime, field, label, Some("layout".into()), resolver);
    }
}

fn dispatch_layout_component_inspector_action(
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    layout_owner: &LayoutOwnerIdentity,
    lifetime: &inspector::LayoutComponentInspectorLifetime,
    (mut workspace, mut inspect_open): (Signal<&'static str>, Signal<bool>),
    pending_edits: Signal<layout_component_edits::LayoutComponentInspectorEdits>,
    action: inspector::LayoutComponentInspectorAction,
) {
    let action_owner = match &action {
        inspector::LayoutComponentInspectorAction::SetPosition { owner, .. }
        | inspector::LayoutComponentInspectorAction::AssignLayout { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetOutline { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetConstraint { owner, .. }
        | inspector::LayoutComponentInspectorAction::RemoveConstraint { owner, .. }
        | inspector::LayoutComponentInspectorAction::ConvertOffsetAxes { owner, .. }
        | inspector::LayoutComponentInspectorAction::NavigateElectrical { owner }
        | inspector::LayoutComponentInspectorAction::SetMirroredHalfState { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetPartMirrored { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetPartAnchorValue { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetPartAnchorFollow { owner, .. }
        | inspector::LayoutComponentInspectorAction::SetPartAnchor { owner, .. }
        | inspector::LayoutComponentInspectorAction::DetachPartAnchor { owner } => owner,
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
        inspector::LayoutComponentInspectorAction::SetPosition {
            owner,
            axis,
            value,
            submitted_draft,
        } => {
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
                Some(submitted_draft),
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
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::SetOutline {
            owner,
            outline,
            submitted_draft,
        } => {
            submit_layout_component_edit(
                runtime,
                pending_edits,
                layout_component_edits::InspectorField::Margin,
                layout_component_edits::outline_resolver(owner.part_id.clone(), outline),
                Some(submitted_draft),
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
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::ConvertOffsetAxes {
            owner,
            constraint_id,
            to,
        } => {
            submit_layout_component_edit(
                runtime,
                pending_edits,
                layout_component_edits::InspectorField::Constraint,
                layout_component_edits::offset_axes_resolver(
                    owner.part_id.clone(),
                    constraint_id,
                    to,
                ),
                None,
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
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::NavigateElectrical { .. } => {
            workspace.set("PCB");
            inspect_open.set(true);
        }
        inspector::LayoutComponentInspectorAction::SetMirroredHalfState {
            owner: _,
            pair_id,
            mirror_id,
            state,
        } => {
            submit_layout_component_edit(
                runtime,
                pending_edits,
                layout_component_edits::InspectorField::MirroredHalfState,
                layout_component_edits::mirrored_half_state_resolver(pair_id, mirror_id, state),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::SetPartMirrored {
            owner: _,
            part_id,
            mirrored,
        } => {
            submit_layout_component_edit(
                runtime,
                pending_edits,
                layout_component_edits::InspectorField::PartMirror,
                layout_component_edits::part_mirror_resolver(part_id, mirrored),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::SetPartAnchorValue { owner, change } => {
            if !part_anchor_value_finite(&change) {
                return;
            }
            submit_layout_component_edit(
                runtime,
                pending_edits,
                layout_component_edits::InspectorField::PartAnchorValue,
                layout_component_edits::part_anchor_value_resolver(owner.part_id, change),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::SetPartAnchorFollow { owner, follow } => {
            submit_layout_component_edit(
                runtime,
                pending_edits,
                layout_component_edits::InspectorField::PartAnchorFollow,
                layout_component_edits::part_anchor_follow_resolver(owner.part_id, follow),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::SetPartAnchor {
            owner,
            matrix_id,
            row,
            column,
            follow,
        } => {
            submit_layout_component_edit(
                runtime,
                pending_edits,
                layout_component_edits::InspectorField::PartAnchorPick,
                layout_component_edits::part_anchor_pick_resolver(
                    owner.part_id,
                    matrix_id,
                    row,
                    column,
                    follow,
                ),
                None,
            );
        }
        inspector::LayoutComponentInspectorAction::DetachPartAnchor { owner } => {
            submit_layout_component_edit(
                runtime,
                pending_edits,
                layout_component_edits::InspectorField::PartAnchorDetach,
                layout_component_edits::part_anchor_detach_resolver(owner.part_id),
                None,
            );
        }
    }
}

/// Ticket 13: finite check for anchor value edits; Follow, Pick and Detach
/// carry no floats.
fn part_anchor_value_finite(change: &boardstudio_core::model::PartAnchorValueChange) -> bool {
    use boardstudio_core::model::PartAnchorValueChange as Change;
    match change {
        Change::Spread(value)
        | Change::Stagger(value)
        | Change::Splay(value)
        | Change::Rotation(value) => value.is_finite(),
        Change::Offset(offset) => offset.x.is_finite() && offset.y.is_finite(),
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
        .and_then(|window| window.match_media("(width < 980px)").ok().flatten())
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
                        .query_selector("#m1-inspector-panel-content > div")
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
