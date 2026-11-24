use super::super::*;
use crate::presentation::editor::camera::*;
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
