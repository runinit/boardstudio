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
    pub(super) keyboard: EventHandler<KeyboardEvent>,
    pub(super) key_up: EventHandler<KeyboardEvent>,
    pub(super) wheel: EventHandler<WheelEvent>,
}

// The editor owns the frame and the resources that survive workspace changes.
// The active workspace supplies its surfaces under the existing dock and canvas shells.
#[derive(Clone)]
pub(super) struct WorkspaceInteraction {
    pub navigation: CanvasNavigationState,
    pub arbiter: CanvasInteractionArbiter,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub inspector_settings: Signal<PanelSettings>,
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
    pub case: Option<case_workspace::CaseFrame>,
    pub parts: Option<parts_workspace::PartsFrame>,
    pub layout: Option<layout_workspace::LayoutFrame>,
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
        _ => rsx! {},
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
pub(super) struct WorkspaceOutputs(pub Signal<Option<(Workspace, WorkspaceSurfaces)>>);

pub(super) fn publish_surfaces(
    mut outputs: WorkspaceOutputs,
    workspace: Workspace,
    surfaces: WorkspaceSurfaces,
) {
    // Only the outlets read this signal. The active workspace publishes before the
    // frame renders its outlets, and can update them independently when its hooks change.
    outputs.0.set(Some((workspace, surfaces)));
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
    let surfaces = outputs.0.read();
    let Some((_, surfaces)) = surfaces.as_ref().filter(|(active, _)| *active == workspace) else {
        return rsx! {};
    };
    match surface {
        Surface::Objects => surfaces.objects.clone(),
        Surface::Inspector => surfaces.inspector.clone(),
        Surface::Toolbar => surfaces.toolbar.clone(),
        Surface::Canvas => surfaces.canvas.clone(),
        Surface::Status => surfaces.status.clone(),
        Surface::Layers => surfaces.layers.clone(),
    }
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
        let actions = camera::camera_actions(
            editor,
            adapter,
            &self.owner,
            &self.scope,
            &self.snapshot,
            &self.svg,
            &footer,
            &bounds,
            &model.selected_part_ids,
        );
        boardstudio_web_ui_shared::ui::command::use_commands(if assembly() {
            Vec::new()
        } else {
            camera_commands(actions).collect()
        });
        let status = if assembly() {
            self.status.summary(model.camera.zoom)
        } else {
            self.status.controls(
                false,
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
        snap_menu_available: bool,
        selection_available: bool,
        zoom_percent: f64,
        model: &ReadModel,
        actions: camera::CameraActions,
    ) -> Element {
        rsx! {
            canvas_status_footer::CanvasStatusFooter {
                snap_settings: self.layout.snap_settings().read().clone(),
                active_part_position: self.part_position,
                snap_menu_available,
                open_menu: self.layout.command_menu(),
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
