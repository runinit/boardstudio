use super::super::*;
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_model::editor::EditorContext;

pub(in crate::presentation) struct SceneBoundsInput<'a> {
    pub enabled: bool,
    pub model: &'a ReadModel,
    pub snapshot: &'a AcceptedSnapshot,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub scene: std::sync::Arc<boardstudio_core::model::SceneDelta>,
    pub render_scope: Scope,
    pub active_workspace: Workspace,
    pub workspace_bounds: Option<(f64, f64, f64, f64)>,
    pub visible_positions: Option<(f64, f64, f64, f64)>,
    pub layout_assembly_3d: Signal<bool>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub zoom_surface_size: Signal<(f64, f64)>,
}

#[derive(Clone)]
pub(in crate::presentation) struct SceneBounds {
    pub accepted_canvas_source: layout_canvas::SceneSource,
    pub canvas_source: layout_canvas::SceneSource,
    pub displayed_layout_bounds: Option<(f64, f64, f64, f64)>,
    pub workspace_rect_bounds: Option<(f64, f64, f64, f64)>,
    pub bounds: (f64, f64, f64, f64),
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub keymap_scale_ratio: f64,
    pub zoom_percent: f64,
    pub width: f64,
    pub height: f64,
    pub view_x: f64,
    pub view_y: f64,
    pub view_box: String,
}

impl SceneBounds {
    pub(in crate::presentation) fn canvas_bounds(&self) -> super::canvas_adapter::CanvasBounds {
        (
            (self.view_x, self.view_y, self.width, self.height),
            self.keymap_scale_ratio,
            (self.min_x, self.max_x, self.min_y, self.max_y),
        )
    }
}

pub(in crate::presentation) fn use_scene_bounds(input: SceneBoundsInput<'_>) -> SceneBounds {
    let SceneBoundsInput {
        enabled,
        model,
        snapshot,
        document,
        scene,
        render_scope,
        active_workspace,
        workspace_bounds,
        visible_positions,
        layout_assembly_3d,
        svg,
        zoom_surface_size,
    } = input;
    let accepted_canvas_source = layout_canvas::SceneSource {
        document: document.clone(),
        scene: snapshot.scene.clone(),
        board_id: render_scope.board_id.clone(),
    };
    let canvas_source = layout_canvas::SceneSource {
        scene: scene.clone(),
        ..accepted_canvas_source.clone()
    };
    let accepted_layout_bounds = use_memo(use_reactive(
        (&accepted_canvas_source, &enabled),
        |(source, enabled)| {
            enabled
                .then(|| {
                    keycaps_fit::layout_canvas_bounds(
                        &source.document,
                        &source.scene,
                        &source.board_id,
                    )
                })
                .flatten()
        },
    ))();
    let displayed_layout_bounds = use_memo(use_reactive(
        (
            &canvas_source,
            &accepted_canvas_source,
            &accepted_layout_bounds,
            &enabled,
        ),
        |(source, accepted_source, accepted_bounds, enabled)| {
            if !enabled {
                return None;
            }
            if source == accepted_source {
                accepted_bounds
            } else {
                keycaps_fit::layout_canvas_bounds(&source.document, &source.scene, &source.board_id)
            }
        },
    ))();
    let workspace_rect_bounds = if active_workspace == Workspace::Layout {
        accepted_layout_bounds
    } else {
        workspace_bounds
    };
    let bounds = workspace_rect_bounds.or(visible_positions);
    let bounds = bounds.unwrap_or((-50.0, 50.0, -50.0, 50.0));
    let bounds = if active_workspace == Workspace::Layout {
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
    let keymap_scale_ratio = if active_workspace == Workspace::Keymap && !layout_assembly_3d() {
        displayed_layout_bounds
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
    SceneBounds {
        accepted_canvas_source,
        canvas_source,
        displayed_layout_bounds,
        workspace_rect_bounds,
        bounds,
        min_x,
        max_x,
        min_y,
        max_y,
        keymap_scale_ratio,
        zoom_percent,
        width,
        height,
        view_x,
        view_y,
        view_box,
    }
}

pub(in crate::presentation) struct FooterBoundsInput<'a> {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub workspace_rect_bounds: Option<(f64, f64, f64, f64)>,
    pub displayed_layout_bounds: Option<(f64, f64, f64, f64)>,
    pub active_workspace: Workspace,
    pub selection_bounds: Option<(f64, f64, f64, f64)>,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub scene: std::sync::Arc<boardstudio_core::model::SceneDelta>,
    pub board_id: String,
    pub model: &'a ReadModel,
}

pub(in crate::presentation) struct FooterBounds {
    pub footer_camera_bounds: (f64, f64, f64, f64),
    pub footer_board_bounds: Option<(f64, f64, f64, f64)>,
    pub footer_selection_bounds: Option<(f64, f64, f64, f64)>,
    pub footer_selection_available: bool,
}

pub(in crate::presentation) fn footer_bounds(input: FooterBoundsInput<'_>) -> FooterBounds {
    let FooterBoundsInput {
        min_x,
        max_x,
        min_y,
        max_y,
        workspace_rect_bounds,
        displayed_layout_bounds,
        active_workspace,
        selection_bounds,
        document,
        scene,
        board_id,
        model,
    } = input;
    let footer_selected_ids = model.selected_part_ids.clone();
    let footer_camera_bounds = if active_workspace == Workspace::Layout {
        // Layout's current camera basis already includes its 12 mm geometry margin and
        // aspect expansion; keymap::fit_camera adds 40 mm to its input to reconstruct
        // the mounted canvas base, so remove that symmetric padding here.
        (min_x + 20.0, max_x - 20.0, min_y + 20.0, max_y - 20.0)
    } else {
        workspace_rect_bounds.unwrap_or((-35.0, 35.0, -25.0, 25.0))
    };
    let footer_board_bounds = if active_workspace == Workspace::Keymap {
        workspace_rect_bounds
    } else {
        displayed_layout_bounds.map(|(min_x, max_x, min_y, max_y)| {
            (min_x + 12.0, max_x - 12.0, min_y + 12.0, max_y - 12.0)
        })
    };
    let footer_selection_bounds = if active_workspace == Workspace::Keymap {
        selection_bounds
    } else {
        keycaps_fit::selected_part_bounds(&document, &scene, &board_id, &footer_selected_ids)
    };
    let footer_selection_available = footer_selection_bounds.is_some();
    FooterBounds {
        footer_camera_bounds,
        footer_board_bounds,
        footer_selection_bounds,
        footer_selection_available,
    }
}

/// Which target a Fit camera action frames: the whole board (falling back to
/// the default board bounds) or the current selection (skipped when gone).
pub(in crate::presentation) enum FitKeymapTarget {
    Board {
        board_bounds: Option<(f64, f64, f64, f64)>,
    },
    Selection {
        selection_ids: Vec<String>,
        selection_bounds: Option<(f64, f64, f64, f64)>,
    },
}

pub(in crate::presentation) struct FitKeymapCameraInput {
    pub layout_owner: LayoutOwnerIdentity,
    pub runtime: Rc<Runtime>,
    pub workspace: Signal<Workspace>,
    pub adapter: SelectionAdapter,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub canvas_bounds: (f64, f64, f64, f64),
    pub target: FitKeymapTarget,
}

/// Shared Fit-board / Fit-selection camera handler: owner and board currency,
/// the selection-identity check for the selection target, then one
/// `keymap::fit_camera` solve.
pub(in crate::presentation) fn fit_keymap_camera_handler(
    input: FitKeymapCameraInput,
) -> EventHandler<()> {
    let FitKeymapCameraInput {
        layout_owner,
        runtime,
        workspace,
        adapter,
        svg,
        canvas_bounds,
        target,
    } = input;
    let owner = layout_owner.clone();
    let camera_runtime = runtime.clone();
    EventHandler::new(move |_| {
        if current_layout_owner(&camera_runtime, workspace, &adapter) != owner {
            return;
        }
        let current = camera_runtime.model();
        if let FitKeymapTarget::Selection { selection_ids, .. } = &target
            && current.selected_part_ids != *selection_ids
        {
            return;
        }
        let Some(scope) = owner.scope.as_ref() else {
            return;
        };
        if !active_board_scope_matches(&current, scope) {
            return;
        }
        let target_bounds = match &target {
            FitKeymapTarget::Board { board_bounds } => {
                board_bounds.unwrap_or((-35.0, 35.0, -25.0, 25.0))
            }
            FitKeymapTarget::Selection {
                selection_bounds, ..
            } => {
                let Some(bounds) = *selection_bounds else {
                    return;
                };
                bounds
            }
        };
        let svg_ref = svg.borrow();
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
        camera_runtime.submit(Event::SetCamera {
            operation_id: camera_runtime.operation(),
            center: camera.center,
            zoom: camera.zoom,
        });
    })
}

pub(in crate::presentation) struct ZoomKeymapInput {
    pub zoom_runtime: Rc<Runtime>,
    pub zoom_workspace: Signal<Workspace>,
    pub zoom_scope: Scope,
    pub zoom_owner: LayoutOwnerIdentity,
    pub zoom_adapter: SelectionAdapter,
    pub zoom_svg: Rc<RefCell<Option<SvgElement>>>,
    pub zoom_token: boardstudio_application::SnapshotToken,
    pub zoom_revision: u64,
    pub zoom_ratio: f64,
    pub zoom_bounds: (f64, f64, f64, f64),
    pub zoom_view: (f64, f64, f64, f64),
    pub render_generation: u64,
}

pub(in crate::presentation) fn zoom_keymap_handler(
    input: ZoomKeymapInput,
) -> impl FnMut(f64) + Clone + 'static {
    let ZoomKeymapInput {
        zoom_runtime,
        zoom_workspace,
        zoom_scope,
        zoom_owner,
        zoom_adapter,
        zoom_svg,
        zoom_token,
        zoom_revision,
        zoom_ratio,
        zoom_bounds,
        zoom_view,
        render_generation,
    } = input;
    move |direction: f64| {
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
        let center = CanvasNavigationState::zoom_center_at(zoom_bounds, location, next_zoom);
        CanvasNavigationState::apply_zoom_target(
            &zoom_runtime,
            boardstudio_web_ui_shared::canvas_navigation::ZoomTarget {
                center,
                zoom: next_zoom,
            },
        );
    }
}

#[derive(Clone, Copy)]
pub(in crate::presentation) struct CameraActions {
    pub fit_board: EventHandler<()>,
    pub fit_selection: EventHandler<()>,
    pub zoom_out: EventHandler<()>,
    pub zoom_in: EventHandler<()>,
}

impl CameraActions {
    pub(in crate::presentation) fn noop() -> Self {
        Self {
            fit_board: EventHandler::new(|_| {}),
            fit_selection: EventHandler::new(|_| {}),
            zoom_out: EventHandler::new(|_| {}),
            zoom_in: EventHandler::new(|_| {}),
        }
    }
}

/// Compact inputs for the shared Fit/zoom camera actions on every 2D canvas.
pub(in crate::presentation) struct CameraActionsInput<'a> {
    pub editor: EditorContext,
    pub adapter: &'a SelectionAdapter,
    pub layout_owner: &'a LayoutOwnerIdentity,
    pub render_scope: &'a Scope,
    pub snapshot: &'a AcceptedSnapshot,
    pub svg: &'a Rc<RefCell<Option<SvgElement>>>,
    pub footer: &'a FooterBounds,
    pub bounds: &'a SceneBounds,
    pub selection_ids: &'a [String],
}

pub(in crate::presentation) fn camera_actions(input: CameraActionsInput<'_>) -> CameraActions {
    let CameraActionsInput {
        editor,
        adapter,
        layout_owner,
        render_scope,
        snapshot,
        svg,
        footer,
        bounds,
        selection_ids,
    } = input;
    let runtime = editor.runtime();
    let workspace = editor.workspace;
    let FooterBounds {
        footer_board_bounds,
        footer_camera_bounds,
        footer_selection_bounds,
        ..
    } = *footer;
    let on_fit_keymap_board = fit_keymap_camera_handler(FitKeymapCameraInput {
        layout_owner: layout_owner.clone(),
        runtime: runtime.clone(),
        workspace,
        adapter: adapter.clone(),
        svg: svg.clone(),
        canvas_bounds: footer_camera_bounds,
        target: FitKeymapTarget::Board {
            board_bounds: footer_board_bounds,
        },
    });
    let on_fit_keymap_selection = fit_keymap_camera_handler(FitKeymapCameraInput {
        layout_owner: layout_owner.clone(),
        runtime: runtime.clone(),
        workspace,
        adapter: adapter.clone(),
        svg: svg.clone(),
        canvas_bounds: footer_camera_bounds,
        target: FitKeymapTarget::Selection {
            selection_ids: selection_ids.to_vec(),
            selection_bounds: footer_selection_bounds,
        },
    });
    let mut zoom_keymap = zoom_keymap_handler(ZoomKeymapInput {
        zoom_runtime: runtime.clone(),
        zoom_workspace: workspace,
        zoom_scope: render_scope.clone(),
        zoom_owner: layout_owner.clone(),
        zoom_adapter: adapter.clone(),
        zoom_svg: svg.clone(),
        zoom_token: snapshot.token,
        zoom_revision: snapshot.document.revision,
        zoom_ratio: bounds.keymap_scale_ratio,
        zoom_bounds: bounds.bounds,
        zoom_view: (bounds.view_x, bounds.view_y, bounds.width, bounds.height),
        render_generation: layout_owner.generation,
    });
    let mut zoom_out = zoom_keymap.clone();
    let on_zoom_keymap_out = EventHandler::new(move |_| zoom_out(-1.0));
    let on_zoom_keymap_in = EventHandler::new(move |_| zoom_keymap(1.0));
    CameraActions {
        fit_board: on_fit_keymap_board,
        fit_selection: on_fit_keymap_selection,
        zoom_out: on_zoom_keymap_out,
        zoom_in: on_zoom_keymap_in,
    }
}

pub(in crate::presentation) fn camera_commands(
    camera: CameraActions,
) -> impl Iterator<Item = boardstudio_web_ui_shared::ui::command::Command> {
    [
        ("Fit board", camera.fit_board),
        ("Zoom out", camera.zoom_out),
        ("Zoom in", camera.zoom_in),
    ]
    .into_iter()
    .map(
        |(label, on_select)| boardstudio_web_ui_shared::ui::command::Command {
            group: "View",
            label: label.into(),
            reason: None,
            hint: None,
            on_select,
        },
    )
}
