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
pub(crate) use boardstudio_web_layout::component_inspector;
mod context_summary;
mod editor;
mod empty_board_canvas;
mod export_workspace;
mod header;
pub(crate) use boardstudio_web_pcb::firmware_positions;
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
mod workbench;
mod workbench_shortcuts;
mod workspace_composition;
mod workspace_navigation;
mod zmk_firmware_export;

use crate::case_generation_lifecycle::AutomaticCaseGeneration;
pub(crate) use boardstudio_web_ui_shared::footprint_graphics;
use canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner};
use canvas_layers::CanvasLayers;
pub(crate) use companion_placement::{
    companion_placement_projection, matrix_context_relations, relation_target_request,
};
pub(in crate::presentation) use component_dispatch::dispatch_layout_component_inspector_action;
pub(crate) use component_dispatch::{
    companion_placement_owner_is_current, layout_component_inspector_owner_key,
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

use crate::runtime::{CoreServices, Runtime};
#[cfg(test)]
use boardstudio_application::SnapshotToken;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, ReadModel, Scope, SelectionMode,
    TerminalOutcome,
};
use boardstudio_core::model::{
    CompanionPlacementTarget, Contour, EditOperation, Finding as LayoutFinding, Part, Position,
    Vec2,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use footprint_graphics::FootprintGraphics;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeSet, HashSet},
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

/// The page hosts the document engine: the web crates below it reach Core's synchronous entry
/// points only through these injected services, so they compile without waiting for the
/// engine. Tests below the page use the matching `boardstudio_web_test_runtime::engine_core_services()` test helper.
pub(crate) fn core_services() -> CoreServices {
    CoreServices {
        validate_open: boardstudio_core::CoreEngine::validate_open,
        open_scratch: || {
            let mut engine = boardstudio_core::CoreEngine::new();
            Box::new(move |request| engine.handle(request))
        },
    }
}

#[component]
fn WorkbenchApp() -> Element {
    let runtime = use_hook(|| Runtime::new(core_services()));
    let runtime = match runtime {
        Ok(runtime) => runtime,
        Err(error) => return rsx! { main { role: "alert", "Browser startup failed: {error}" } },
    };
    let workbench::RuntimeBundle {
        version,
        project_created,
        pending_new_keyboard,
        new_keyboard_error,
        adapter,
    } = workbench::use_workbench_runtime(runtime.clone());
    let workbench::PanelsBundle {
        workspace,
        project_browser,
        return_workspace,
        observed_version,
    } = workbench::use_workbench_panels(
        runtime.clone(),
        version,
        project_created,
        pending_new_keyboard,
        new_keyboard_error,
        adapter.clone(),
    );
    workbench::use_keyboard_creation(
        runtime.clone(),
        observed_version,
        pending_new_keyboard,
        project_created,
        new_keyboard_error,
    );
    let preference_warning = workbench::use_workbench_appearance();
    let workbench::HeaderBundle {
        project_name,
        save_state,
        mut settings_open,
        palette,
        on_undo,
        on_redo,
        on_export,
    } = workbench::use_workbench_header(
        runtime.clone(),
        workspace,
        return_workspace,
        project_browser,
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
                on_retry_save: {
                    let runtime = runtime.clone();
                    move |_| runtime.submit(boardstudio_application::Event::RetrySave { operation_id: runtime.operation() })
                },
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
