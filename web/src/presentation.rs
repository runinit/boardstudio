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
mod companion_placement;
mod component_dispatch;
mod component_inspector;
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
mod routing;
pub(crate) use boardstudio_web_case::shared_viewer;
pub(crate) use boardstudio_web_layout::setup_guide;
mod finding_navigation;
mod notifications;
mod tree_cell_anchor;
mod view_helpers;
mod workbench_shortcuts;
mod workspace_composition;
mod workspace_navigation;
mod zmk_firmware_export;

use crate::case_generation_lifecycle::AutomaticCaseGeneration;
pub(crate) use boardstudio_web_ui_shared::footprint_graphics;
use canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner};
use canvas_layers::CanvasLayers;
pub(crate) use companion_placement::{
    companion_placement_projection, layout_relationship_summary, matrix_context_relations,
    part_anchor_summary, relation_target_request,
};
pub(crate) use component_dispatch::{
    companion_placement_owner_is_current, layout_component_inspector_owner_key,
};
pub(in crate::presentation) use component_dispatch::{
    dispatch_layout_component_inspector_action, submit_layout_component_edit,
};
pub(crate) use component_inspector::{
    layout_component_inspector_projection, mirrored_half_projection,
};
use editor::Editor;
#[cfg(test)]
pub(crate) use export_workspace::ExportPanel;
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
pub(crate) use notifications::RuntimeReportBanner;
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
pub(crate) use routing::WorkspaceRoutingSlots;
use selection::{ReentrancyReset, SelectionAdapter};
use setup_guide::{PendingNewKeyboard, SetupGuideRequest, SetupGuideStage};
pub(crate) use setup_project_name::{SetupProjectNameEdit, use_setup_project_name};
pub(crate) use tree_cell_anchor::{
    OwnedTreeCellAnchor, tree_cell_anchor_for_owner, update_tree_cell_anchor,
};
pub(crate) use view_helpers::{
    design_consumer_view_mode_handler, focus_first_inspector_control_on_next_frame,
    is_compact_viewport, matrix_snap_parameters, pin_inspector_on_desktop,
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

fn durability_state(durability: &Durability) -> &'static str {
    match durability {
        Durability::Saved { .. } => "saved",
        Durability::Saving { .. } => "saving",
        Durability::Failed { .. } => "failed",
        _ => "pending",
    }
}
