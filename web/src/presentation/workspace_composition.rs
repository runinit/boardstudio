//! Private routing for the six independently owned workspace surfaces.
use super::workbench::editor_session::commands::camera_commands;
use super::workbench::editor_session::{camera, keyboard};
use super::*;
use boardstudio_web_ui_model::editor::EditorContext;

#[derive(Clone)]
pub(super) struct SharedObjectsInput {
    pub(super) selected_context: Signal<Option<objects::ScopedTreeContext>>,
    pub(super) on_select: EventHandler<objects::TreeSelectRequest>,
    pub(super) on_navigate: EventHandler<(Scope, String, Option<String>)>,
    pub(super) on_nudge: EventHandler<objects::TreeNudgeRequest>,
    pub(super) board_setup: objects::BoardSetupMount,
}

#[derive(Clone)]
pub(super) struct CanvasEventHandlers {
    pub(super) mount: EventHandler<MountedEvent>,
    pub(super) start_pan: EventHandler<PointerEvent>,
    pub(super) move_pointer: EventHandler<PointerEvent>,
    pub(super) end_pointer: EventHandler<PointerEvent>,
    pub(super) cancel_pointer: EventHandler<PointerEvent>,
    pub(super) lost_capture: EventHandler<PointerEvent>,
    pub(super) keyboard: EventHandler<KeyboardEvent>,
    pub(super) key_up: EventHandler<KeyboardEvent>,
    pub(super) wheel: EventHandler<WheelEvent>,
}

// The editor owns the frame and the resources that survive workspace changes.
// The active workspace supplies its surfaces under the existing dock and canvas shells.
#[derive(Clone)]
pub(super) struct WorkspaceInteraction {
    pub navigation: CanvasNavigationState,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub inspector_settings: Signal<PanelSettings>,
}

/// Session-owned services the Layout workspace consumes. Unlike the deleted
/// `WorkspacePayload::Layout` prop bag, these are raw shared mounts — Layout
/// assembles its own Objects/Inspector/toolbar/canvas inputs inside
/// `LayoutWorkspace` instead of receiving them prebuilt from the session.
#[derive(Clone)]
pub(super) struct LayoutSessionServices {
    pub matrix_inspector: objects::MatrixInspectorMount,
    pub matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub matrix_placement: objects::MatrixPlacementMount,
    pub mirrored_pair: objects::MirroredPairMount,
    pub outline_inspector: Option<outline_lifecycle::OutlineInspectorProjection>,
    pub layout_findings: boardstudio_web_layout::layout_findings_state::LayoutFindingsState,
    pub on_return_from_layout_finding: Callback<(), ()>,
    pub on_close_layout_findings: Callback<(), ()>,
    pub on_layout_finding: Callback<layout_findings::Request, ()>,
    pub visible_ids: Rc<Vec<String>>,
    pub objects_panel_settings: Signal<PanelSettings>,
}

pub(super) struct WorkspaceFrame {
    pub scope: Scope,
    pub snapshot: AcceptedSnapshot,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub scene: std::sync::Arc<boardstudio_core::model::SceneDelta>,
    pub model: Rc<ReadModel>,
    pub visible_positions: Option<(f64, f64, f64, f64)>,
    pub owner: LayoutOwnerIdentity,
    pub objects: SharedObjectsInput,
    pub handlers: CanvasEventHandlers,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub surface_size: Signal<(f64, f64)>,
    pub matrix_setup: objects::MatrixSetupMount,
    pub part_placement: part_placement::PartPlacementMount,
    pub placement_error: Option<String>,
    pub layout_target: Signal<Option<String>>,
    pub parts_query: parts::PartsQuery,
    pub browse_parts: EventHandler<()>,
    pub select_module: EventHandler<String>,
    pub show_position_inspector: bool,
    pub camera: camera::SceneBounds,
    pub status: StatusFooter,
    pub payload: WorkspacePayload,
}

/// The per-workspace inputs that only one workspace surface consumes. Typed
/// by workspace so a surface can never observe another workspace's frame.
pub(super) enum WorkspacePayload {
    Case(case_workspace::CaseFrame),
    Parts(parts_workspace::PartsFrame),
    Layout(LayoutSessionServices),
    Pcb,
    Keymap,
    Keycaps,
    Export,
}

impl WorkspaceFrame {
    /// Session-owned services the Layout workspace consumes. The payload is
    /// `Layout` whenever `LayoutWorkspace` renders, so this never observes
    /// another workspace's frame.
    pub(super) fn layout_services(&self) -> &LayoutSessionServices {
        let WorkspacePayload::Layout(services) = &self.payload else {
            panic!("layout services require the Layout workspace payload");
        };
        services
    }
}

/// Clear the tree selection and submit the empty part selection. Shared by the
/// workspace surfaces that reset selection without choosing a new target.
pub(super) fn clear_selection(runtime: &Rc<Runtime>, adapter: &SelectionAdapter) {
    let mut selected_context = adapter.selected_context;
    selected_context.set(None);
    let mut anchor_scope = adapter.anchor_scope;
    anchor_scope.set(None);
    runtime.submit(boardstudio_application::Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: Vec::new(),
        range_part_ids: Vec::new(),
        mode: boardstudio_application::SelectionMode::Replace,
    });
}

/// The live tree selection for a workspace scope, dropping contexts the
/// accepted revision no longer resolves.
pub(super) fn current_tree_context(
    adapter: &SelectionAdapter,
    model: &ReadModel,
    scope: &Scope,
) -> Option<objects::ScopedTreeContext> {
    adapter.selected_context.read().clone().filter(|selected| {
        selected.scope == *scope
            && selection::context_is_current(model, &selected.scope, &selected.context)
    })
}

/// Whether a tree context resolves to exactly the selected part: the gate
/// behind the single-component Position inspector on every 2D surface.
pub(super) fn selection_is_single_component(
    model: &ReadModel,
    context: &objects::TreeContext,
) -> bool {
    selection::resolve_context(model, context)
        .is_some_and(|ids| ids.len() == 1 && model.selected_part_ids.as_slice() == ids.as_slice())
        && matches!(context, objects::TreeContext::Component { .. })
}

/// The document parts on the active board for canvas bounds and anchor math.
pub(super) fn board_visible_parts<'a>(
    document: &'a boardstudio_core::model::ProjectDoc,
    active_board_id: &str,
) -> Vec<&'a boardstudio_core::model::Part> {
    let board_part_ids: std::collections::HashSet<&str> = document
        .boards
        .iter()
        .find(|board| board.id == active_board_id)
        .into_iter()
        .flat_map(|board| board.part_ids.iter().map(String::as_str))
        .collect();
    document
        .parts
        .iter()
        .filter(|part| board_part_ids.contains(part.id.as_str()))
        .collect()
}

#[derive(Clone)]
pub(super) struct Frame(Rc<WorkspaceFrame>);

impl Frame {
    pub(super) fn new(frame: WorkspaceFrame) -> Self {
        Self(Rc::new(frame))
    }
}

impl PartialEq for Frame {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[component]
pub(super) fn RoutedWorkspace(workspace: Workspace, frame: ReadSignal<Frame>) -> Element {
    use_context_provider(|| frame);
    match workspace {
        Workspace::Case => rsx! { case_workspace::CaseWorkspace {} },
        Workspace::Parts => rsx! { parts_workspace::PartsWorkspace {} },
        Workspace::Layout => rsx! { layout_workspace::LayoutWorkspace {} },
        Workspace::Pcb => rsx! { pcb_workspace::PcbWorkspace {} },
        Workspace::Keymap => rsx! { keymap_workspace::KeymapWorkspace {} },
        Workspace::Keycaps => rsx! { keycaps_workspace::KeycapsWorkspace {} },
        Workspace::Export => rsx! {},
    }
}

pub(super) fn use_frame() -> Rc<WorkspaceFrame> {
    use_context::<ReadSignal<Frame>>().read().0.clone()
}

#[derive(Clone, PartialEq)]
pub(super) struct WorkspaceSurfaces {
    pub objects: Element,
    pub inspector: Element,
    pub toolbar: Element,
    pub canvas: Element,
    pub status: Element,
    pub layers: Element,
}

#[derive(Clone, Copy)]
pub(super) struct WorkspaceOutputs {
    pub objects: Signal<Option<(Workspace, Element)>>,
    pub inspector: Signal<Option<(Workspace, Element)>>,
    pub toolbar: Signal<Option<(Workspace, Element)>>,
    pub canvas: Signal<Option<(Workspace, Element)>>,
    pub status: Signal<Option<(Workspace, Element)>>,
    pub layers: Signal<Option<(Workspace, Element)>>,
}

pub(super) fn use_workspace_outputs() -> WorkspaceOutputs {
    WorkspaceOutputs {
        objects: use_signal(|| None),
        inspector: use_signal(|| None),
        toolbar: use_signal(|| None),
        canvas: use_signal(|| None),
        status: use_signal(|| None),
        layers: use_signal(|| None),
    }
}

/// Publish the active workspace's surfaces after render so signal writes never
/// happen mid-render. Because `Element` (`VNode`) compares by `Rc::ptr_eq`,
/// any re-render of the workspace replaces all six surface elements together.
pub(super) fn use_publish_surfaces(workspace: Workspace, surfaces: WorkspaceSurfaces) {
    let outputs = use_context::<WorkspaceOutputs>();
    use_effect(use_reactive(
        (&workspace, &surfaces),
        move |(workspace, surfaces)| {
            for (mut signal, element) in [
                (outputs.objects, surfaces.objects),
                (outputs.inspector, surfaces.inspector),
                (outputs.toolbar, surfaces.toolbar),
                (outputs.canvas, surfaces.canvas),
                (outputs.status, surfaces.status),
                (outputs.layers, surfaces.layers),
            ] {
                let next = Some((workspace, element));
                if *signal.peek() != next {
                    signal.set(next);
                }
            }
        },
    ));
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Surface {
    Objects,
    Inspector,
    Toolbar,
    Canvas,
    Status,
    Layers,
}

#[component]
pub(super) fn WorkspaceOutlet(workspace: Workspace, surface: Surface) -> Element {
    let outputs = use_context::<WorkspaceOutputs>();
    let signal = match surface {
        Surface::Objects => outputs.objects,
        Surface::Inspector => outputs.inspector,
        Surface::Toolbar => outputs.toolbar,
        Surface::Canvas => outputs.canvas,
        Surface::Status => outputs.status,
        Surface::Layers => outputs.layers,
    };
    let published = signal.read();
    match published
        .as_ref()
        .filter(|(active, _)| *active == workspace)
    {
        Some((_, element)) => element.clone(),
        None => rsx! {},
    }
}

pub(super) fn use_toolbar_selection_availability(
    tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    layout_owner: LayoutOwnerIdentity,
    accepted_canvas_source: layout_canvas::SceneSource,
    snapshot: &AcceptedSnapshot,
    model: &ReadModel,
    selected_tree_context: Option<objects::ScopedTreeContext>,
    active_workspace: Option<Workspace>,
) -> (bool, bool) {
    let retained_toolbar_cell = tree_cell_anchor_for_owner(&tree_cell_anchor, &layout_owner);
    use_memo(use_reactive(
        (
            &accepted_canvas_source,
            &snapshot.token,
            &snapshot.session_epoch,
            &model.active_instance_id,
            &selected_tree_context,
            &retained_toolbar_cell,
            &active_workspace,
        ),
        |(source, token, session_epoch, active_instance_id, selected, retained, workspace)| {
            if !matches!(workspace, Some(Workspace::Layout | Workspace::Pcb)) {
                return (false, false);
            }
            let model = ReadModel {
                accepted: Some(AcceptedSnapshot {
                    token,
                    session_epoch,
                    document: source.document,
                    scene: source.scene,
                }),
                active_board_id: source.board_id,
                active_instance_id,
                ..ReadModel::default()
            };
            let supports = |kind| {
                selected.as_ref().is_some_and(|selected| {
                    objects::context_for_selection_kind(
                        &model,
                        &selected.context,
                        kind,
                        retained.as_ref(),
                    )
                    .is_some()
                })
            };
            (
                supports(objects::LayoutSelectionKind::Column),
                supports(objects::LayoutSelectionKind::Row),
            )
        },
    ))()
}

/// Shared design-canvas key selection for the Keymap and Keycaps surfaces:
/// both admit a click only while their workspace, scope and generation own
/// the live selection, clear on an empty id, and open the Inspector for a
/// key that still exists after the selection lands. `is_current` carries
/// each workspace's extra currency guard and `key_exists` its projection.
pub(super) fn design_key_select_handler(
    editor: EditorContext,
    adapter: SelectionAdapter,
    frame: &WorkspaceFrame,
    expected_workspace: Workspace,
    require_snapshot_before_clear: bool,
    is_current: impl Fn(&ReadModel, &Scope) -> bool + 'static,
    key_exists: impl Fn(&ReadModel, &Scope, &str) -> bool + 'static,
) -> EventHandler<String> {
    let runtime = editor.runtime();
    let workspace = editor.workspace;
    let scope = frame.scope.clone();
    let generation = frame.owner.generation;
    let CompactPanelState {
        mut objects_open,
        inspector_open: mut inspect_open,
    } = editor.panels;
    EventHandler::new(move |key_id: String| {
        if workspace() != expected_workspace
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let model = runtime.model();
        if !is_current(&model, &scope) {
            return;
        }
        if key_id.is_empty() {
            if !require_snapshot_before_clear || model.accepted.is_some() {
                clear_selection(&runtime, &adapter);
            }
            return;
        }
        if !key_exists(&model, &scope, &key_id) {
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
            boardstudio_application::SelectionMode::Replace,
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
        if !is_current(&current, &scope)
            || !selection::context_is_current(&current, &scope, &context)
            || !key_exists(&current, &scope, &key_id)
        {
            return;
        }
        objects_open.set(false);
        inspect_open.set(true);
    })
}

pub(super) fn toggle_footprints(layers: LayerVisibility) -> EventHandler<()> {
    let mut footprints = layers.footprints;
    EventHandler::new(move |()| footprints.set(!footprints()))
}

pub(super) fn design_toolbar(
    frame: &WorkspaceFrame,
    editor: EditorContext,
    adapter: &SelectionAdapter,
    detail: &str,
    contextual_tools: Element,
) -> Element {
    let assembly = frame.status.layout.assembly_3d();
    let layers = editor.layers;
    rsx! {
        boardstudio_web_ui_shared::design_view_toolbar::DesignViewToolbar {
            contextual_tools: Some(contextual_tools),
            workspace: frame.owner.workspace,
            detail: detail.to_owned(),
            assembly_3d: assembly(),
            footprints_visible: (layers.footprints)() && !(layers.hidden)().contains("Footprints"),
            on_view_mode: design_consumer_view_mode_handler(editor.runtime(), editor.workspace, adapter.clone(), frame.owner.clone(), assembly),
            on_toggle_footprints: toggle_footprints(layers),
        }
    }
}

impl WorkspaceFrame {
    pub(super) fn use_design_canvas(
        &self,
        editor: EditorContext,
        adapter: &SelectionAdapter,
        model: &ReadModel,
        workspace_bounds: Option<(f64, f64, f64, f64)>,
        selection_bounds: Option<(f64, f64, f64, f64)>,
    ) -> (String, CanvasEventHandlers, Element) {
        let workspace = self.owner.workspace;
        let assembly = self.status.layout.assembly_3d();
        let bounds = camera::use_scene_bounds(camera::SceneBoundsInput {
            enabled: true,
            model,
            snapshot: &self.snapshot,
            document: self.document.clone(),
            scene: self.scene.clone(),
            render_scope: self.scope.clone(),
            active_workspace: workspace,
            workspace_bounds,
            visible_positions: self.visible_positions,
            layout_assembly_3d: assembly,
            svg: self.svg.clone(),
            zoom_surface_size: self.surface_size,
        });
        let footer = camera::footer_bounds(camera::FooterBoundsInput {
            min_x: bounds.min_x,
            max_x: bounds.max_x,
            min_y: bounds.min_y,
            max_y: bounds.max_y,
            workspace_rect_bounds: bounds.workspace_rect_bounds,
            displayed_layout_bounds: bounds.displayed_layout_bounds,
            active_workspace: workspace,
            selection_bounds,
            document: self.document.clone(),
            scene: self.scene.clone(),
            board_id: self.scope.board_id.clone(),
            model,
        });
        let actions = camera::camera_actions(camera::CameraActionsInput {
            editor,
            adapter,
            layout_owner: &self.owner,
            render_scope: &self.scope,
            snapshot: &self.snapshot,
            svg: &self.svg,
            footer: &footer,
            bounds: &bounds,
            selection_ids: &model.selected_part_ids,
        });
        boardstudio_web_ui_shared::ui::command::use_commands(if assembly() {
            Vec::new()
        } else {
            camera_commands(actions).collect()
        });
        let status = if assembly() {
            self.status.summary(model.camera.zoom)
        } else {
            self.status.controls(
                footer.footer_selection_available,
                bounds.zoom_percent,
                model,
                actions,
            )
        };
        let handlers = CanvasEventHandlers {
            wheel: EventHandler::new(keyboard::build_wheel(
                editor,
                adapter,
                &self.scope,
                self.owner.generation,
                &self.svg,
                &bounds,
            )),
            ..self.handlers.clone()
        };
        (bounds.view_box, handlers, status)
    }

    pub(super) fn design_canvas(
        &self,
        keycaps_fit: Option<keycaps_fit::KeycapsFitState>,
        canvas: Element,
    ) -> Element {
        if (self.status.layout.assembly_3d())() {
            rsx! { layout_viewer::LayoutCanonicalViewer { keycaps_fit } }
        } else {
            canvas
        }
    }
}

pub(super) struct StatusFooter {
    pub count: usize,
    pub toggle_findings: Callback<()>,
    pub layout: layout_workspace::LayoutWorkspaceState,
    pub part_position: Option<Vec2>,
}

impl StatusFooter {
    pub(super) fn controls(
        &self,
        selection_available: bool,
        zoom_percent: f64,
        model: &ReadModel,
        actions: camera::CameraActions,
    ) -> Element {
        rsx! {
            canvas_status_footer::CanvasStatusFooter {
                active_part_position: self.part_position,
                board_available: true,
                selection_available,
                zoom_percent,
                findings_count: Some(self.count),
                on_toggle_findings: self.toggle_findings,
                on_fit_board: actions.fit_board,
                on_fit_selection: actions.fit_selection,
                on_zoom_out: actions.zoom_out,
                on_zoom_in: actions.zoom_in,
            }
            if let Some(guide) = model.snap_guide.as_ref() {
                span { class: "ml-auto max-w-[min(35vw,340px)] truncate", role: "status", "{guide.label}" }
            }
        }
    }

    pub(super) fn summary(&self, zoom: f64) -> Element {
        rsx! {
            span { "{zoom * 100.0:.0}%" }
            canvas_status_footer::FindingsButton { count: self.count, on_toggle: self.toggle_findings }
        }
    }
}
