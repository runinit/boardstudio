//! Keycaps-owned workspace surface composition.
use super::keycaps_fit::KeycapsFitInspector;
use super::keycaps_scene::{KeycapsCanvas, KeycapsKeyList, KeycapsView};
use super::keycaps_settings::{
    KeycapsBoardSettingsEditor, KeycapsMatrixSettingsEditor, KeycapsSettingsActions,
    KeycapsSettingsEditor, SelectedKeySettings,
};
use super::objects;
use super::workspace_composition::{CanvasEventHandlers, SharedObjectsInput};
use super::workspace_composition::{
    WorkspaceInteraction, WorkspaceSurfaces, design_key_select_handler, design_toolbar, use_frame,
    use_publish_surfaces,
};
use super::*;
use super::{keycaps_finding_marker, keycaps_navigation};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::Contour;
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_model::finding_navigation::{
    FindingNavigationRequest, FindingNavigationTarget, KeycapsFitSource, KeycapsFitState,
    layout_camera_basis,
};
use boardstudio_web_ui_shared::inspector_fields::InspectorSection;
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

pub(super) fn use_projection(
    runtime: Rc<Runtime>,
    accepted_token: Option<SnapshotToken>,
    current_scope: Option<Scope>,
    active_board_id: String,
) -> Option<(Rc<KeycapsView>, Rc<[Contour]>)> {
    use_memo(use_reactive(
        (&accepted_token, &current_scope, &active_board_id),
        move |(token, scope, board_id)| {
            let model = runtime.model();
            let snap = model.accepted.as_ref()?;
            if token.as_ref() != Some(&snap.token) {
                return None;
            }
            let view = super::keycaps_scene::project(snap, scope.as_ref()?, &board_id)?;
            Some((view, super::accepted_board_contours(snap, &board_id)))
        },
    ))
    .read()
    .clone()
}

#[derive(Clone)]
pub(super) struct KeycapsWorkspaceState {
    focused_finding: Signal<Option<keycaps_finding_marker::FocusedFinding>>,
    pub fit: Signal<Option<KeycapsFitState>>,
    pending_layout_fit: Signal<Option<keycaps_navigation::PendingLayoutFit>>,
    navigation_alive: Rc<std::cell::Cell<bool>>,
}

pub(super) fn use_keycaps_workspace_state(
    workspace: Workspace,
    scope: Option<Scope>,
    token: Option<SnapshotToken>,
    revision: Option<u64>,
    active_board_id: String,
) -> KeycapsWorkspaceState {
    let focused_finding = use_signal(|| None);
    let pending_layout_fit = use_signal(|| None);
    keycaps_finding_marker::use_retire_stale_finding(
        focused_finding,
        workspace,
        scope,
        token,
        revision,
        active_board_id,
    );
    KeycapsWorkspaceState {
        focused_finding,
        fit: use_signal(|| None),
        pending_layout_fit,
        navigation_alive: keycaps_navigation::use_navigation_lifetime(),
    }
}

impl KeycapsWorkspaceState {
    pub(super) fn focused_finding(&self) -> Signal<Option<keycaps_finding_marker::FocusedFinding>> {
        self.focused_finding
    }
    pub(super) fn navigation_alive(&self) -> Rc<std::cell::Cell<bool>> {
        self.navigation_alive.clone()
    }
    pub(super) fn set_focused_finding(
        &self,
        finding: Option<keycaps_finding_marker::FocusedFinding>,
    ) {
        let mut s = self.focused_finding;
        s.set(finding);
    }
    pub(super) fn focus_finding(
        &self,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        finding_id: String,
    ) {
        let navigation_id =
            keycaps_finding_marker::next_navigation_id(self.focused_finding.peek().as_ref());
        self.set_focused_finding(Some(keycaps_finding_marker::FocusedFinding {
            scope,
            token,
            revision,
            finding_id,
            navigation_id,
        }));
    }
    pub(super) fn set_pending_layout_fit(
        &self,
        pending: Option<keycaps_navigation::PendingLayoutFit>,
    ) {
        let mut s = self.pending_layout_fit;
        s.set(pending);
    }
    pub(super) fn use_pending_layout_fit<T: Clone + PartialEq + 'static>(
        &self,
        observed_owner: T,
        current_owner: impl Fn() -> keycaps_navigation::LiveNavigationOwner + 'static,
        resolve_geometry: impl Fn(
            &keycaps_navigation::PendingLayoutFit,
        ) -> Option<keycaps_navigation::DestinationFitGeometry>
        + 'static,
        perform: impl Fn(keycaps_navigation::FitAction, keycaps_navigation::NavigationOwner) + 'static,
    ) {
        keycaps_navigation::use_pending_layout_fit(
            self.pending_layout_fit,
            observed_owner,
            self.navigation_alive.clone(),
            current_owner,
            resolve_geometry,
            perform,
        );
    }
}

pub(super) struct CanvasInput {
    pub(super) view: Option<Rc<KeycapsView>>,
    pub(super) contours: Rc<[Contour]>,
    pub(super) view_box: String,
    pub(super) selected_ids: BTreeSet<String>,
    pub(super) handlers: CanvasEventHandlers,
    pub(super) on_select_key: EventHandler<String>,
}

pub(super) struct InspectorInput {
    pub(super) view: Option<Rc<KeycapsView>>,
    pub(super) document: Rc<boardstudio_core::model::ProjectDoc>,
    pub(super) selected_key_id: Option<String>,
    pub(super) on_select_key: EventHandler<String>,
    pub(super) settings_editor: Option<(SelectedKeySettings, KeycapsSettingsActions)>,
    pub(super) settings_actions: Option<KeycapsSettingsActions>,
    pub(super) fit_state: Option<KeycapsFitState>,
    pub(super) mechanical_layer_ids: Rc<[String]>,
    pub(super) fit_retry: EventHandler<()>,
    pub(super) fit_navigate: EventHandler<FindingNavigationRequest>,
}

pub(super) struct NavigationInput {
    pub(super) shared: SharedObjectsInput,
    pub(super) view: Option<Rc<KeycapsView>>,
    pub(super) settings_actions: Option<KeycapsSettingsActions>,
}

pub(super) fn objects(input: NavigationInput) -> Element {
    let s = input.shared;
    rsx! {
        objects::Objects {
            selected_context: s.selected_context, on_select: s.on_select, on_navigate: s.on_navigate,
            on_nudge: s.on_nudge, board_setup: Some(s.board_setup), matrix_setup: None, pair_created: None,
            on_place_component: None, layout_target: None, parts_query: None, on_browse_parts: None,
            placement_error: None, matrix_inspector: None,
        }
        if let (Some(view), Some(actions)) = (input.view, input.settings_actions) {
            div { class: "max-h-[35%] flex-none overflow-auto border-t border-border px-4 py-3 max-[760px]:max-h-none",
                KeycapsBoardSettingsEditor { settings: view.board_settings.clone(), actions }
            }
        }
    }
}

pub(super) fn canvas(input: CanvasInput) -> Element {
    let Some(view) = input.view else {
        return rsx! { p { role: "status", "Keycaps are unavailable for the current board." } };
    };
    let h = input.handlers;
    rsx! {
        section { class: "flex min-h-0 min-w-0 flex-1", "aria-label": "Keycaps workspace",
            svg {
                class: "block h-full min-h-[250px] min-w-0 w-full flex-1 touch-none bg-canvas max-[760px]:min-h-0",
                view_box: "{input.view_box}", preserve_aspect_ratio: "xMidYMid meet", tabindex: "0", role: "group",
                "aria-label": "Physical keycaps; select a key with click, Enter, or Space, hold Space and drag to pan, or use the mouse wheel to zoom",
                onmounted: h.mount, onpointerdown: h.start_pan, onpointermove: h.move_pointer,
                onpointerup: h.end_pointer, onpointercancel: h.cancel_pointer, onlostpointercapture: h.lost_capture,
                onkeydown: h.keyboard, onkeyup: h.key_up, onwheel: h.wheel,
                g { transform: "scale(1,-1)",
                    KeycapsCanvas { view, contours: input.contours, selected_ids: input.selected_ids, on_select_key: input.on_select_key }
                }
            }
        }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    let Some(view) = input.view else {
        return rsx! { section { class: "grid min-w-0 gap-2 p-3 text-foreground", "aria-label": "Selected keycap", h2 { "Keycaps unavailable" } p { "The current board has no Keycaps projection." } } };
    };
    let selected_title = input
        .selected_key_id
        .as_deref()
        .and_then(|id| view.keys.iter().find(|k| k.id.as_ref() == id))
        .map_or_else(
            || "Select a key".to_owned(),
            |k| format!("{} · key", k.reference),
        );
    rsx! {
        section { class: "flex min-w-0 flex-col gap-3 px-6 py-4 text-foreground", role: "region", "aria-label": "Keycaps inspector",
            header { class: "flex items-baseline justify-between gap-2 pb-3",
                h2 { class: "text-2xl! font-semibold!", role: "heading", "Keycaps" }
                span { class: "whitespace-nowrap text-sm text-muted-foreground", "{view.assigned_count}/{view.keys.len()} assigned" }
            }
            if view.keys.is_empty() { p { role: "status", "Add switches in Layout to configure keycaps." } }
            if let Some(actions) = input.settings_actions {
                InspectorSection { title: "Matrix profiles", default_open: false,
                    section { class: "grid min-w-0 gap-2 p-3 text-foreground", "aria-label": "Matrix profiles",
                        for m in view.matrices.iter() {
                            KeycapsMatrixSettingsEditor { key: "{m.id}", matrix_id: m.id.to_string(), matrix_name: m.name.to_string(), settings: m.settings.clone(), actions: actions.clone() }
                        }
                        if view.matrices.is_empty() { p { "Standalone switches use their individual profile override." } }
                    }
                }
            }
            InspectorSection { title: selected_title, default_open: true,
                div { class: "grid min-w-0 gap-3 pb-5",
                    KeycapsKeyList { view: view.clone(), selected_key_id: input.selected_key_id, on_select_key: input.on_select_key }
                    if let Some((selected, actions)) = input.settings_editor { KeycapsSettingsEditor { selected, actions } }
                }
            }
            KeycapsFitInspector {
                document: input.document, state: input.fit_state, mechanical_layer_ids: input.mechanical_layer_ids,
                on_retry: input.fit_retry, on_navigate: input.fit_navigate,
            }
        }
    }
}

use boardstudio_web_ui_model::editor::EditorContext;

pub(super) fn use_clearance_assessment(
    editor: EditorContext,
    model: &ReadModel,
    current_scope: Option<&Scope>,
) -> keycaps_fit::KeycapsFitActions {
    let runtime = editor.runtime();
    let navigation = use_context::<KeycapsWorkspaceState>();
    let keycaps_fit_source = current_scope
        .filter(|scope| active_board_scope_matches(model, scope))
        .and_then(|scope| {
            model.accepted.as_ref().map(|snapshot| {
                let case_preview_current = runtime.cad_scene().is_some_and(|scene| {
                    scene.exact
                        && scene.scope == *scope
                        && scene.token == snapshot.token
                        && scene.prepared.revision == snapshot.document.revision
                });
                KeycapsFitSource {
                    scope: scope.clone(),
                    token: snapshot.token,
                    revision: snapshot.document.revision,
                    case_preview_current,
                }
            })
        });
    keycaps_fit::use_keycaps_fit_with_state(runtime, keycaps_fit_source, navigation.fit)
}

#[component]
pub(super) fn KeycapsWorkspace() -> Element {
    let editor = use_context::<EditorContext>();
    let adapter = use_context::<SelectionAdapter>();
    let frame = use_frame();
    let interaction = use_context::<WorkspaceInteraction>();
    let runtime = editor.runtime();
    let workspace = editor.workspace;
    let instance_selection = editor.instance_selection;
    let model = &runtime.model();
    let current_scope = &frame.owner.scope;
    let fit = use_clearance_assessment(editor, model, current_scope.as_ref());
    let navigation = use_context::<KeycapsWorkspaceState>();
    let focused_finding = navigation.focused_finding();
    let case_selection = use_context::<shared_viewer::CaseSelection>();
    let on_finding = on_keycaps_finding_handler(OnKeycapsFindingInput {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        scope: frame.scope.clone(),
        generation: frame.owner.generation,
        workspace,
        objects_open: editor.panels.objects_open,
        inspect_open: editor.panels.inspector_open,
        body_selection: case_selection.body,
        case_selection_for_owner: case_selection,
        navigation_alive: navigation.navigation_alive(),
        inspector_settings: interaction.inspector_settings,
        fit_state: fit.state.clone(),
        keycaps_state: navigation,
        source_surface: frame.svg.clone(),
        select_tree: frame.objects.on_select,
        case_selection,
    });
    let document = &frame.document;
    let keycaps_edit_source = current_scope
        .as_ref()
        .filter(|scope| {
            active_board_scope_matches(model, scope) && instance_selection.is_current(model)
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
    let keycaps_settings_actions = keycaps_settings::use_keycaps_settings_actions(
        runtime.clone(),
        keycaps_edit_source,
        workspace,
        adapter.generation,
    );
    let projection = use_projection(
        runtime.clone(),
        frame.owner.token,
        current_scope.clone(),
        model.active_board_id.clone(),
    );
    let (keycaps_view, contours) = projection.map_or_else(
        || (None, Rc::from([])),
        |(view, contours)| (Some(view), contours),
    );
    let on_select_key = design_key_select_handler(
        editor,
        adapter.clone(),
        &frame,
        Workspace::Keycaps,
        false,
        move |model, scope| {
            active_board_scope_matches(model, scope) && instance_selection.is_current(model)
        },
        |model, scope, key_id| {
            model
                .accepted
                .as_ref()
                .and_then(|snapshot| {
                    keycaps_scene::project(snapshot, scope, &model.active_board_id)
                })
                .is_some_and(|view| view.keys.iter().any(|key| key.id.as_ref() == key_id))
        },
    );
    let (view_box, handlers, status) = frame.use_design_canvas(
        editor,
        &adapter,
        model,
        keycaps_view
            .as_deref()
            .and_then(|view| keycaps_bounds(view, &contours)),
        None,
    );
    let toolbar = design_toolbar(
        &frame,
        editor,
        &adapter,
        "Profiles, legends & fit",
        rsx! {
            workspace_navigation::InspectorActions {
                label: "Keycaps tools", workspace, inspect_open: editor.panels.inspector_open, inspector_settings: interaction.inspector_settings,
                generation: adapter.generation, actions: vec![("Profile", "[aria-label^='Keycap overrides for '] [data-slot='select-trigger'][aria-label^='Profile override for ']", false), ("Legend", "[aria-label^='Keycap overrides for '] [aria-label^='Legend for ']", false), ("Color", "[aria-label^='Keycap overrides for '] [aria-label^='Keycap color for ']", false)],
                disabled: model.selected_part_ids.is_empty(),
            }
        },
    );
    let selected_key_id = model.selected_part_ids.first().cloned();
    let mechanical_layer_ids = current_scope
        .as_ref()
        .zip(model.accepted.as_ref())
        .and_then(|(scope, snapshot)| current_case_layer_ids(&runtime, scope, snapshot))
        .unwrap_or_else(|| Rc::from([]));
    let settings_editor = keycaps_view.as_deref().and_then(|view| {
        let selected =
            keycaps_settings::project_selected_key(document, view, selected_key_id.as_deref())?;
        keycaps_settings_actions
            .clone()
            .map(|actions| (selected, actions))
    });
    let surfaces = WorkspaceSurfaces {
        objects: objects(NavigationInput {
            shared: frame.objects.clone(),
            view: keycaps_view.clone(),
            settings_actions: keycaps_settings_actions.clone(),
        }),
        inspector: inspector(InspectorInput {
            view: keycaps_view.clone(),
            document: Rc::new((**document).clone()),
            selected_key_id,
            on_select_key,
            settings_editor,
            settings_actions: keycaps_settings_actions,
            fit_state: fit.state.clone(),
            mechanical_layer_ids,
            fit_retry: fit.on_retry,
            fit_navigate: on_finding,
        }),
        toolbar,
        canvas: if (frame.status.layout.assembly_3d())() {
            // Keycaps 3D mounts the shared <Viewport3D /> host through the
            // Keycaps crate viewer, with the focused finding wired to its
            // camera focus (the generic design-canvas 3D path drops it).
            rsx! {
                keycaps_viewer::KeycapsAssemblyViewer {
                    keycaps_fit: fit.state.clone(),
                    focused_finding: focused_finding(),
                }
            }
        } else {
            frame.design_canvas(
                fit.state,
                canvas(CanvasInput {
                    view: keycaps_view,
                    contours,
                    view_box,
                    selected_ids: model.selected_part_ids.iter().cloned().collect(),
                    handlers,
                    on_select_key,
                }),
            )
        },
        status,
        layers: rsx! {},
    };
    use_publish_surfaces(Workspace::Keycaps, surfaces);
    rsx! {}
}

pub(super) struct OnKeycapsFindingInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub scope: Scope,
    pub generation: u64,
    pub workspace: Signal<Workspace>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub body_selection: Signal<Option<shared_viewer::BodySelection>>,
    pub case_selection_for_owner: shared_viewer::CaseSelection,
    pub navigation_alive: Rc<Cell<bool>>,
    pub inspector_settings: Signal<PanelSettings>,
    pub fit_state: Option<KeycapsFitState>,
    pub keycaps_state: keycaps_workspace::KeycapsWorkspaceState,
    pub source_surface: Rc<RefCell<Option<SvgElement>>>,
    pub select_tree: EventHandler<objects::TreeSelectRequest>,
    pub case_selection: shared_viewer::CaseSelection,
}

pub(super) fn on_keycaps_finding_handler(
    input: OnKeycapsFindingInput,
) -> EventHandler<FindingNavigationRequest> {
    let OnKeycapsFindingInput {
        runtime,
        adapter,
        scope,
        generation,
        mut workspace,
        mut objects_open,
        mut inspect_open,
        mut body_selection,
        case_selection_for_owner,
        navigation_alive,
        inspector_settings,
        fit_state,
        keycaps_state,
        source_surface,
        select_tree,
        case_selection,
    } = input;
    EventHandler::new(move |request: FindingNavigationRequest| {
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
                FindingNavigationTarget::Part { part_id, .. } => {
                    objects::component_context_for_finding_part(&model, part_id)
                }
                _ => None,
            },
        ) else {
            return;
        };
        let focused = keycaps_navigation::focused_finding_for_admitted_route(&request, &admitted);
        let effects = admitted.effects;
        let navigation_owner = admitted.owner;
        let source_basis = if workspace() == Workspace::Keycaps {
            source_surface.borrow().as_ref().and_then(|surface| {
                let rect = surface.get_bounding_client_rect();
                layout_camera_basis(
                    &snapshot.document,
                    &snapshot.scene,
                    &request.source.scope.board_id,
                    (rect.width(), rect.height()),
                )
            })
        } else {
            None
        };
        keycaps_state.set_focused_finding(focused);
        let focus_runtime = runtime.clone();
        let focus_adapter = adapter.clone();
        let focus_body_selection = body_selection;
        let focus_case_selection = case_selection_for_owner;
        let focus_workspace = workspace;
        keycaps_state.set_pending_layout_fit(None);
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
                    body_selection.set(Some(shared_viewer::BodySelection { scope, body_id }));
                }
                keycaps_navigation::RouteAction::CloseObjects => objects_open.set(false),
                keycaps_navigation::RouteAction::OpenInspector => inspect_open.set(true),
                keycaps_navigation::RouteAction::PinInspector => {
                    pin_inspector_on_desktop(inspector_settings);
                }
                keycaps_navigation::RouteAction::QueueLayoutFit(request) => {
                    keycaps_state.set_pending_layout_fit(Some(*request));
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
    })
}
