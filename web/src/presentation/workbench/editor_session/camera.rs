use super::super::*;
use super::EditorContext;
use boardstudio_web_ui_model::Workspace;

pub(in crate::presentation) struct SvgSurface {
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub zoom_surface_size: Signal<(f64, f64)>,
}

pub(in crate::presentation) fn use_svg_surface(
    canvas_navigation: &CanvasNavigationState,
    workspace: Signal<Workspace>,
) -> SvgSurface {
    let svg = use_hook(|| Rc::new(RefCell::new(None::<SvgElement>)));
    let zoom_surface_size = canvas_navigation.surface_size();
    // Read actual post-layout dimensions so attached controls keep their screen size
    // when the viewport or either dock changes, without polling or a resize loop.
    let geometry_observer = use_hook(|| {
        Rc::new(RefCell::new(
            None::<(
                web_sys::ResizeObserver,
                Closure<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>,
            )>,
        ))
    });
    use_effect({
        let svg = svg.clone();
        let observer = geometry_observer.clone();
        move || {
            let _ = zoom_surface_size();
            let _ = workspace();
            let Some(surface) = svg.borrow().clone() else {
                return;
            };
            if let Some((previous, _)) = observer.borrow_mut().take() {
                previous.disconnect();
            }
            let measured = surface.clone();
            let mut size = zoom_surface_size;
            let callback =
                Closure::<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>::new(move |_, _| {
                    let rect = measured.get_bounding_client_rect();
                    let next = (rect.width(), rect.height());
                    if next != *size.peek() {
                        size.set(next);
                    }
                });
            if let Ok(next) = web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref()) {
                next.observe(&surface);
                *observer.borrow_mut() = Some((next, callback));
            }
        }
    });
    use_drop(move || {
        if let Some((observer, _)) = geometry_observer.borrow_mut().take() {
            observer.disconnect();
        }
    });
    SvgSurface {
        svg,
        zoom_surface_size,
    }
}

/// The shared canvas mount handler plus the placement-focus effect. Every
/// workspace canvas needs these at session lifetime; Layout-only inspector
/// projections moved into `LayoutWorkspace`.
pub(in crate::presentation) fn use_session_canvas_mount(
    svg: &Rc<RefCell<Option<SvgElement>>>,
    zoom_surface_size: Signal<(f64, f64)>,
    runtime: Rc<Runtime>,
    part_placement: &part_placement::PartPlacementSession,
    matrix_placement: &objects::MatrixPlacementSession,
) -> EventHandler<MountedEvent> {
    let mount = EventHandler::new(CanvasNavigationState::mount_handler(
        zoom_surface_size,
        runtime.clone(),
        svg.clone(),
        part_placement.projection.is_some(),
    ));
    let placement_active = part_placement.projection.is_some()
        || part_placement.busy
        || matrix_placement.placement.is_some()
        || matrix_placement.busy;
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
    mount
}

/// The session-level position-inspector flag carried on the frame for PCB.
/// Layout computes the full version inside `LayoutWorkspace`; the board and
/// component projections it also consults only ever project while Layout is
/// active, so the selection check alone is exact for every other workspace.
pub(in crate::presentation) fn session_show_position_inspector(
    model: &ReadModel,
    selected_tree_context: Option<&objects::ScopedTreeContext>,
) -> bool {
    selected_tree_context.is_none_or(|selected| {
        super::super::super::workspace_composition::selection_is_single_component(
            model,
            &selected.context,
        )
    })
}

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

pub(in crate::presentation) struct PlacementBindingsInput<'a> {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub camera_center: Vec2,
    pub runtime: Rc<Runtime>,
    pub workspace: Signal<Workspace>,
    pub adapter: SelectionAdapter,
    pub version: Signal<u64>,
    pub layout_selection_kind: Signal<objects::LayoutSelectionKind>,
    pub guide_preferences: Signal<Option<setup_guide::SetupGuidePreferences>>,
    pub parts_query: parts::PartsQuery,
    pub parts_selection: parts::PartsSelection,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub layout_target: Signal<Option<String>>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub matrix_setup: &'a mut objects::MatrixSetupMount,
    pub mirrored_pair: objects::MirroredPairSession,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
}

pub(in crate::presentation) struct PlacementBindings {
    pub part_placement: part_placement::PartPlacementSession,
    pub interaction_version: Signal<u64>,
    pub observed_interaction_version: u64,
}

pub(in crate::presentation) fn use_placement_bindings(
    input: PlacementBindingsInput<'_>,
) -> PlacementBindings {
    let canvas_interaction = use_context::<LayoutInteraction>();
    let PlacementBindingsInput {
        min_x,
        max_x,
        min_y,
        max_y,
        camera_center,
        runtime,
        workspace,
        adapter,
        version,
        layout_selection_kind,
        guide_preferences,
        parts_query,
        parts_selection,
        layout_snap_settings,
        layout_target,
        objects_open,
        inspect_open,
        matrix_setup,
        mirrored_pair,
        svg,
    } = input;
    let canvas_center =
        part_placement::canvas_world_center(min_x, max_x, min_y, max_y, camera_center);
    let part_placement = part_placement::use_part_placement(part_placement::PartPlacementHost {
        runtime: runtime.clone(),
        load_definition: Rc::new(|document, definition_id| {
            Box::pin(
                async move { parts::load_component_definition(&document, &definition_id).await },
            )
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
    let interaction_version = use_signal(|| 0_u64);
    let observed_interaction_version = interaction_version() + canvas_interaction.version();
    PlacementBindings {
        part_placement,
        interaction_version,
        observed_interaction_version,
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

pub(in crate::presentation) fn visible_positions(
    visible: &[&Part],
) -> Option<(f64, f64, f64, f64)> {
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
}

pub(in crate::presentation) struct CamNamed {
    pub svg_surface: SvgSurface,
    pub scene_bounds: SceneBounds,
    pub footer_bounds: FooterBounds,
    pub placement: PlacementBindings,
    pub actions: CameraActions,
}

pub(in crate::presentation) fn use_camera_bundle(
    state: &super::session_state::SessionState,
    views: &super::setup_workspaces::Views,
    snapshot: &AcceptedSnapshot,
    render_scope: &Scope,
    visible_positions: Option<(f64, f64, f64, f64)>,
    matrix_setup: &mut objects::MatrixSetupMount,
) -> CamNamed {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let runtime = &editor.runtime();
    let adapter = &state.adapter;
    let workspace = editor.workspace;
    let layout_owner = &state.view.layout_owner;
    let model = &state.view.model;
    let document = &views.document;
    let scene = &views.scene;
    let active_workspace = state.active_workspace;
    let layout_assembly_3d = state.layout_state.assembly_3d();
    let version = editor.version;
    let layout_selection_kind = state.layout_state.selection_kind();
    let guide_preferences = state.setup_state.guide_preferences();
    let parts_query = state.view.parts_query;
    let parts_selection = state.view.parts_selection;
    let layout_snap_settings = state.layout_state.snap_settings();
    let layout_target = state.view.layout_target;
    let objects_open = editor.panels.objects_open;
    let inspect_open = editor.panels.inspector_open;
    let mirrored_pair = &state.matrix_pair.mirrored_pair;
    let canvas_navigation = &state.canvas_navigation;
    let SvgSurface {
        svg,
        zoom_surface_size,
    } = use_svg_surface(canvas_navigation, workspace);

    // Keymap and Keycaps derive their own bounds, footer and camera actions
    // through the design-canvas path, so the session bundle skips the
    // duplicate solve while either owns the canvas.
    let design_canvas_workspace =
        matches!(active_workspace, Workspace::Keymap | Workspace::Keycaps);
    let scene_bounds = use_scene_bounds(SceneBoundsInput {
        enabled: !design_canvas_workspace,
        model,
        snapshot,
        document: document.clone(),
        scene: scene.clone(),
        render_scope: render_scope.clone(),
        active_workspace,
        workspace_bounds: (active_workspace == Workspace::Pcb)
            .then(|| pcb_bounds(snapshot, render_scope))
            .flatten(),
        visible_positions,
        layout_assembly_3d,
        svg: svg.clone(),
        zoom_surface_size,
    });
    let footer = if design_canvas_workspace {
        FooterBounds {
            footer_camera_bounds: scene_bounds.bounds,
            footer_board_bounds: None,
            footer_selection_bounds: None,
            footer_selection_available: false,
        }
    } else {
        footer_bounds(FooterBoundsInput {
            min_x: scene_bounds.min_x,
            max_x: scene_bounds.max_x,
            min_y: scene_bounds.min_y,
            max_y: scene_bounds.max_y,
            workspace_rect_bounds: scene_bounds.workspace_rect_bounds,
            displayed_layout_bounds: scene_bounds.displayed_layout_bounds,
            active_workspace,
            selection_bounds: None,
            document: document.clone(),
            scene: scene.clone(),
            board_id: render_scope.board_id.clone(),
            model,
        })
    };
    let actions = if design_canvas_workspace {
        CameraActions::noop()
    } else {
        camera_actions(CameraActionsInput {
            editor,
            adapter,
            layout_owner,
            render_scope,
            snapshot,
            svg: &svg,
            footer: &footer,
            bounds: &scene_bounds,
            selection_ids: &model.selected_part_ids,
        })
    };
    let PlacementBindings {
        part_placement,
        interaction_version,
        observed_interaction_version,
    } = use_placement_bindings(PlacementBindingsInput {
        min_x: scene_bounds.min_x,
        max_x: scene_bounds.max_x,
        min_y: scene_bounds.min_y,
        max_y: scene_bounds.max_y,
        camera_center: model.camera.center,
        runtime: runtime.clone(),
        workspace,
        adapter: adapter.clone(),
        version,
        layout_selection_kind,
        guide_preferences,
        parts_query,
        parts_selection,
        layout_snap_settings,
        layout_target,
        objects_open,
        inspect_open,
        matrix_setup: &mut *matrix_setup,
        mirrored_pair: mirrored_pair.clone(),
        svg: svg.clone(),
    });
    CamNamed {
        svg_surface: SvgSurface {
            svg,
            zoom_surface_size,
        },
        scene_bounds,
        footer_bounds: footer,
        placement: PlacementBindings {
            part_placement,
            interaction_version,
            observed_interaction_version,
        },
        actions,
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
