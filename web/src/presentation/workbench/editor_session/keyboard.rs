use super::super::*;
use super::EditorContext;
use boardstudio_web_ui_model::Workspace;

fn keyboard_handler(
    mirrored_pair: objects::MirroredPairMount,
    placement: part_placement::PartPlacementMount,
    matrix_placement: objects::MatrixPlacementMount,
    mut transform_tool: Signal<Option<objects::LayoutTransformTool>>,
    snap_settings: Signal<objects::LayoutSnapSettings>,
) -> impl FnMut(KeyboardEvent) + Clone + 'static {
    let interaction = use_context::<LayoutInteraction>();
    move |event: KeyboardEvent| {
        let key = event.key();
        let delta = match key {
            Key::ArrowLeft => Vec2 { x: -1.0, y: 0.0 },
            Key::ArrowRight => Vec2 { x: 1.0, y: 0.0 },
            Key::ArrowDown => Vec2 { x: 0.0, y: -1.0 },
            Key::ArrowUp => Vec2 { x: 0.0, y: 1.0 },
            Key::Enter | Key::Escape => Vec2::default(),
            _ => return,
        };
        let enter = key == Key::Enter;
        let step = |pitch: f64, default: f64| {
            let fraction = snap_settings.peek().snap_fraction;
            if fraction < 0.0 {
                -fraction
            } else if fraction > 0.0 {
                pitch * fraction
            } else {
                default
            }
        };
        let position = |center: Vec2, pitch: Vec2, default: Vec2| Vec2 {
            x: center.x + delta.x * step(pitch.x, default.x),
            y: center.y + delta.y * step(pitch.y, default.y),
        };
        let center = |at: Vec2, pitch: Vec2| {
            if enter {
                return at;
            }
            position(
                at,
                pitch,
                Vec2 {
                    x: pitch.x * 0.25,
                    y: pitch.y * 0.25,
                },
            )
        };
        match (interaction.current(), key) {
            (None, Key::Escape) if transform_tool.peek().is_some() => transform_tool.set(None),
            (_, Key::Escape) | (None, _) => return,
            (Some(CanvasMode::PlacingMatrix), _) => {
                let Some(active) = matrix_placement.placement.as_ref() else {
                    return;
                };
                let request = objects::MatrixPlacementMove {
                    owner: active.owner.clone(),
                    center: center(active.matrix.origin, active.matrix.pitch),
                };
                if enter {
                    matrix_placement.on_commit
                } else {
                    matrix_placement.on_move
                }
                .call(request);
            }
            (Some(CanvasMode::PlacingMirroredPair), _) => {
                let Some(active) = mirrored_pair.placement.as_ref() else {
                    return;
                };
                let request = objects::MirroredPairMove {
                    owner: active.owner.clone(),
                    center: center(active.pair.center, active.pair.matrix.pitch),
                };
                if enter {
                    mirrored_pair.on_commit
                } else {
                    mirrored_pair.on_move
                }
                .call(request);
            }
            (Some(CanvasMode::PlacingPart), _) => {
                if let Some(active) = placement.projection.as_ref() {
                    if enter {
                        placement.on_commit.call(active.pending.at);
                    } else {
                        placement.on_move.call(position(
                            active.pending.at,
                            Vec2 { x: 19.05, y: 19.05 },
                            Vec2 { x: 0.1, y: 0.1 },
                        ));
                    }
                }
            }
            _ => return,
        }
        event.prevent_default();
    }
}

pub(super) struct StartPanInput {
    pub navigation: CanvasNavigationState,
    pub render_scope: Scope,
    pub render_generation: u64,
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub adapter: SelectionAdapter,
    pub mirrored_pair: objects::MirroredPairMount,
    pub pending_splay_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub transform_inspector: objects::MatrixTransformInspectorMount,
    pub layout_owner_for_pick: LayoutOwnerIdentity,
    pub workspace: Signal<Workspace>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
}

pub(super) fn start_pan_handler(
    input: StartPanInput,
) -> impl FnMut(PointerEvent) + Clone + 'static {
    let canvas_interaction = use_context::<LayoutInteraction>();
    let StartPanInput {
        navigation,
        render_scope,
        render_generation,
        runtime,
        svg,
        adapter,
        mirrored_pair,
        mut pending_splay_pick,
        transform_inspector,
        layout_owner_for_pick,
        workspace,
        view_x,
        view_y,
        width,
        height,
    } = input;
    move |event: PointerEvent| {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        if pointer.default_prevented() || pointer.button() != 0 {
            return;
        }
        canvas_interaction.sync_control(pointer.ctrl_key());
        let active_workspace = workspace();
        if active_workspace != Workspace::Layout {
            if matches!(
                active_workspace,
                Workspace::Pcb | Workspace::Keymap | Workspace::Keycaps
            ) && canvas_interaction.space_down()
                && canvas_interaction.current().is_none()
                && runtime.scope().as_ref() == Some(&render_scope)
                && (adapter.generation)() == render_generation
                && runtime.with_model(|model| model.gesture.is_none())
                && navigation.drag_interaction().borrow().is_none()
            {
                super::canvas_adapter::start_pan(
                    &runtime,
                    &navigation,
                    &svg,
                    &render_scope,
                    render_generation,
                    &pointer,
                    canvas_interaction,
                );
                pointer.prevent_default();
            }
            return;
        }
        let id = pointer.pointer_id();
        if canvas_interaction
            .pointer()
            .is_some_and(|owner| owner != id)
        {
            return;
        }
        match canvas_interaction.current() {
            Some(mode @ CanvasMode::PlacingMirroredPair) => {
                canvas_interaction.handle(CanvasEvent::PointerDown {
                    mode,
                    id,
                    client: client_point(&pointer),
                    control: pointer.ctrl_key(),
                });
                if let Some(placement) = mirrored_pair.placement.as_ref() {
                    let owner = placement.owner.clone();
                    if runtime.scope().as_ref() != Some(&owner.scope)
                        || (adapter.generation)() != owner.scope_generation
                    {
                        mirrored_pair.on_cancel.call(owner);
                    } else if let Some(center) =
                        coordinates(&svg, &pointer, view_x, view_y, width, height)
                    {
                        mirrored_pair
                            .on_commit
                            .call(objects::MirroredPairMove { owner, center });
                    }
                }
            }
            Some(CanvasMode::PickingSplayOrigin) => {
                if let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height) {
                    canvas_interaction.end(CanvasMode::PickingSplayOrigin);
                    apply_pending_splay_origin_pick(
                        &runtime,
                        workspace,
                        &adapter,
                        &layout_owner_for_pick,
                        PendingSplayOriginPick {
                            pending: &mut pending_splay_pick,
                            inspector: &transform_inspector,
                            point,
                            swing: pointer.ctrl_key(),
                        },
                    );
                }
            }
            Some(mode) => {
                canvas_interaction.handle(CanvasEvent::PointerDown {
                    mode,
                    id: pointer.pointer_id(),
                    client: client_point(&pointer),
                    control: pointer.ctrl_key(),
                });
            }
            None => {
                if !layout_owner_is_current(&runtime, workspace, &adapter, &layout_owner_for_pick)
                    || runtime.with_model(|model| model.gesture.is_some())
                    || navigation.drag_interaction().borrow().is_some()
                {
                    return;
                }
                if !canvas_interaction.space_down() {
                    return;
                }
                super::canvas_adapter::start_pan(
                    &runtime,
                    &navigation,
                    &svg,
                    &render_scope,
                    render_generation,
                    &pointer,
                    canvas_interaction,
                );
            }
        }
        pointer.prevent_default();
    }
}

pub(super) struct WheelInput {
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub scope: Scope,
    pub adapter: SelectionAdapter,
    pub render_generation: u64,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
    pub keymap_scale_ratio: f64,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

pub(super) fn wheel_handler(input: WheelInput) -> impl FnMut(WheelEvent) + Clone + 'static {
    let WheelInput {
        runtime,
        svg,
        scope,
        adapter,
        render_generation,
        view_x,
        view_y,
        width,
        height,
        keymap_scale_ratio,
        min_x,
        max_x,
        min_y,
        max_y,
    } = input;
    // Wheel bursts (trackpads emit dozens of ticks per gesture) coalesce
    // into one zoom application per animation frame: ticks only add to
    // this burst, and the single rAF flush submits once. Without this,
    // every tick ran Runtime::submit -> changed()/notify() plus a DOM
    // rect read, re-rendering the full scene per tick (Issue 2).
    let pending = Rc::new(RefCell::new(layout_camera::WheelZoomCoalescer::new()));
    move |event: WheelEvent| {
        if runtime.scope().as_ref() != Some(&scope) || (adapter.generation)() != render_generation {
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
        let Some(window) = web_sys::window() else {
            return;
        };
        let schedule_flush = pending.borrow_mut().push(
            wheel.delta_y(),
            layout_camera::WheelAnchor {
                world: location.world,
                x_fraction: location.x_fraction,
                y_fraction: location.y_fraction,
            },
        );
        if !schedule_flush {
            // A flush is already queued for this frame; this tick rides it.
            return;
        }
        let flush_pending = pending.clone();
        let flush_runtime = runtime.clone();
        let flush_scope = scope.clone();
        let flush_generation = adapter.generation;
        // keymap_scale_ratio is 1.0 outside Keymap, so Layout and Keymap
        // share the reference 0.25..4 limits.
        let flush = move || {
            let start_zoom = flush_runtime.model().camera.zoom;
            // Replay the burst tick-by-tick through the clamp: a burst
            // that hits a limit mid-burst then backs off ends exactly
            // where sequential application ends.
            let final_zoom = flush_pending
                .borrow()
                .coalesced_zoom(start_zoom, keymap_scale_ratio);
            let Some(batch) = flush_pending.borrow_mut().take_batch() else {
                return;
            };
            let Some(final_zoom) = final_zoom else {
                return;
            };
            if final_zoom == start_zoom {
                return;
            }
            if flush_runtime.scope().as_ref() != Some(&flush_scope)
                || (flush_generation)() != render_generation
            {
                return;
            }
            CanvasNavigationState::apply_zoom_target(
                &flush_runtime,
                boardstudio_web_ui_shared::canvas_navigation::ZoomTarget {
                    center: CanvasNavigationState::zoom_center_at(
                        (min_x, max_x, min_y, max_y),
                        PointerLocation {
                            world: batch.anchor.world,
                            x_fraction: batch.anchor.x_fraction,
                            y_fraction: batch.anchor.y_fraction,
                        },
                        final_zoom,
                    ),
                    zoom: final_zoom,
                },
            );
        };
        let callback = Closure::once_into_js(flush);
        let _ = window.request_animation_frame(callback.unchecked_ref());
    }
}

pub(super) fn build_keyboard_bundle(
    state: &super::session_state::SessionState,
    cam: &super::camera::CamNamed,
    render_scope: &Scope,
) -> (
    impl FnMut(KeyboardEvent) + Clone + 'static,
    impl FnMut(KeyboardEvent) + Clone + 'static,
    impl FnMut(PointerEvent) + Clone + 'static,
) {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let runtime = &editor.runtime();
    let svg = &cam.svg_surface.svg;
    let mirrored_pair = &state.matrix_pair.mirrored_pair;
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &state.matrix_pair.matrix_placement;
    let layout_snap_settings = state.layout_state.snap_settings();
    let pending_splay_origin_pick = state.layout_state.pending_splay_origin_pick();
    let matrix_transform_inspector = &state.matrix_transform_inspector;
    let canvas_navigation = &state.canvas_navigation;
    let adapter = &state.adapter;
    let layout_owner = &state.view.layout_owner;
    let workspace = editor.workspace;
    let view_x = cam.scene_bounds.view_x;
    let view_y = cam.scene_bounds.view_y;
    let width = cam.scene_bounds.width;
    let height = cam.scene_bounds.height;
    let render_generation = state.render_generation;
    let keyboard = keyboard_handler(
        mirrored_pair.clone(),
        part_placement.clone(),
        matrix_placement.clone(),
        state.layout_state.transform_tool(),
        layout_snap_settings,
    );
    let input = use_context::<layout_input::LayoutInput>();
    let key_up = move |event| input.keyup(event);
    let start_pan = start_pan_handler(StartPanInput {
        navigation: canvas_navigation.clone(),
        render_scope: render_scope.clone(),
        render_generation,
        runtime: runtime.clone(),
        svg: svg.clone(),
        adapter: adapter.clone(),
        mirrored_pair: mirrored_pair.clone(),
        pending_splay_pick: pending_splay_origin_pick,
        transform_inspector: matrix_transform_inspector.clone(),
        layout_owner_for_pick: layout_owner.clone(),
        workspace,
        view_x,
        view_y,
        width,
        height,
    });
    (keyboard, key_up, start_pan)
}

pub(in crate::presentation) fn build_wheel(
    editor: EditorContext,
    adapter: &SelectionAdapter,
    scope: &Scope,
    generation: u64,
    svg: &Rc<RefCell<Option<SvgElement>>>,
    bounds: &super::camera::SceneBounds,
) -> impl FnMut(WheelEvent) + Clone + 'static {
    wheel_handler(WheelInput {
        runtime: editor.runtime(),
        svg: svg.clone(),
        scope: scope.clone(),
        adapter: adapter.clone(),
        render_generation: generation,
        view_x: bounds.view_x,
        view_y: bounds.view_y,
        width: bounds.width,
        height: bounds.height,
        keymap_scale_ratio: bounds.keymap_scale_ratio,
        min_x: bounds.min_x,
        max_x: bounds.max_x,
        min_y: bounds.min_y,
        max_y: bounds.max_y,
    })
}
