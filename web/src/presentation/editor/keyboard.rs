use super::super::*;
use super::EditorContext;
use boardstudio_web_ui_model::Workspace;

pub(super) struct KeyboardInput {
    pub runtime: Rc<Runtime>,
    pub drag: Rc<RefCell<Option<Drag>>>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub render_scope: Scope,
    pub space_down: Rc<Cell<bool>>,
    pub mirrored_pair: objects::MirroredPairMount,
    pub placement: part_placement::PartPlacementMount,
    pub matrix_placement: objects::MatrixPlacementMount,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub snap_settings: Signal<objects::LayoutSnapSettings>,
    pub pending_splay_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub pending_anchor_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub transform_inspector: objects::MatrixTransformInspectorMount,
    pub interaction_version: Signal<u64>,
}

pub(super) fn keyboard_handler(
    input: KeyboardInput,
) -> impl FnMut(KeyboardEvent) + Clone + 'static {
    let KeyboardInput {
        runtime,
        drag,
        svg,
        render_scope,
        space_down,
        mirrored_pair,
        placement,
        matrix_placement,
        canvas_interaction,
        snap_settings,
        mut pending_splay_pick,
        mut pending_anchor_pick,
        transform_inspector,
        interaction_version,
    } = input;
    move |event: KeyboardEvent| {
        let key = event.data().key().to_string();
        let code = event.data().code().to_string();
        let modifiers = event.data().modifiers();
        if canvas_interaction.is_owner(CanvasInteractionOwner::MatrixPlacement) {
            let Some(active) = matrix_placement.placement.as_ref() else {
                if key == "Escape"
                    && let Some(owner) = matrix_placement.cancel_owner.clone()
                {
                    event.prevent_default();
                    matrix_placement.on_cancel.call(owner);
                } else {
                    event.prevent_default();
                }
                return;
            };
            let owner = active.owner.clone();
            if key == "Escape" {
                event.prevent_default();
                matrix_placement.on_cancel.call(owner);
                return;
            }
            if key == "Enter" {
                event.prevent_default();
                matrix_placement
                    .on_commit
                    .call(objects::MatrixPlacementMove {
                        owner,
                        center: active.matrix.origin,
                    });
                return;
            }
            let (dx, dy) = match key.as_str() {
                "ArrowLeft" => (-1.0, 0.0),
                "ArrowRight" => (1.0, 0.0),
                "ArrowDown" => (0.0, -1.0),
                "ArrowUp" => (0.0, 1.0),
                _ => (0.0, 0.0),
            };
            if dx != 0.0 || dy != 0.0 {
                event.prevent_default();
                let fraction = snap_settings.read().snap_fraction;
                let step = |pitch: f64| {
                    if fraction.is_sign_negative() {
                        -fraction
                    } else {
                        pitch * if fraction == 0.0 { 0.25 } else { fraction }
                    }
                };
                matrix_placement.on_move.call(objects::MatrixPlacementMove {
                    owner,
                    center: Vec2 {
                        x: active.matrix.origin.x + dx * step(active.matrix.pitch.x),
                        y: active.matrix.origin.y + dy * step(active.matrix.pitch.y),
                    },
                });
                return;
            }
        }
        if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair)
            && let Some(placement) = mirrored_pair.placement.as_ref()
        {
            let owner = placement.owner.clone();
            if key == "Escape" {
                event.prevent_default();
                mirrored_pair.on_cancel.call(owner);
                return;
            }
            if key == "Enter" {
                event.prevent_default();
                mirrored_pair.on_commit.call(objects::MirroredPairMove {
                    owner,
                    center: placement.pair.center,
                });
                return;
            }
            let (dx, dy) = match key.as_str() {
                "ArrowLeft" => (-1.0, 0.0),
                "ArrowRight" => (1.0, 0.0),
                "ArrowDown" => (0.0, -1.0),
                "ArrowUp" => (0.0, 1.0),
                _ => (0.0, 0.0),
            };
            if dx != 0.0 || dy != 0.0 {
                event.prevent_default();
                let settings = snap_settings.read();
                let fraction = settings.snap_fraction;
                let pitch = placement.pair.matrix.pitch;
                let step = |axis_pitch: f64| {
                    if fraction.is_sign_negative() {
                        -fraction
                    } else {
                        axis_pitch * if fraction == 0.0 { 0.25 } else { fraction }
                    }
                };
                mirrored_pair.on_move.call(objects::MirroredPairMove {
                    owner,
                    center: Vec2 {
                        x: placement.pair.center.x + dx * step(pitch.x),
                        y: placement.pair.center.y + dy * step(pitch.y),
                    },
                });
                return;
            }
        }
        if key == "Escape"
            && canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair)
            && let Some(form) = mirrored_pair.form.as_ref()
            && mirrored_pair.placement.is_none()
        {
            event.prevent_default();
            mirrored_pair.on_cancel.call(form.owner.clone());
            return;
        }
        if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair) {
            return;
        }
        if canvas_interaction.is_owner(CanvasInteractionOwner::PartPlacement) {
            if let Some(active) = placement.projection.as_ref() {
                if key == "Escape" {
                    event.prevent_default();
                    placement.on_cancel.call(());
                } else if key == "Enter" {
                    event.prevent_default();
                    placement.on_commit.call(active.pending.at);
                } else if matches!(
                    key.as_str(),
                    "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight"
                ) {
                    event.prevent_default();
                    let mut at = active.pending.at;
                    let fraction = snap_settings.read().snap_fraction;
                    let step = if fraction < 0.0 {
                        -fraction
                    } else if fraction > 0.0 {
                        19.05 * fraction
                    } else {
                        0.1
                    };
                    match key.as_str() {
                        "ArrowUp" => at.y += step,
                        "ArrowDown" => at.y -= step,
                        "ArrowLeft" => at.x -= step,
                        "ArrowRight" => at.x += step,
                        _ => {}
                    }
                    placement.on_move.call(at);
                }
            } else if key == "Escape" {
                event.prevent_default();
                placement.on_cancel.call(());
            } else {
                event.prevent_default();
            }
            return;
        }
        if key == "Escape" && pending_splay_pick.peek().is_some() {
            pending_splay_pick.set(None);
            event.prevent_default();
            return;
        }
        if key == "Escape" && pending_anchor_pick.peek().is_some() {
            pending_anchor_pick.set(None);
            transform_inspector.clear_aggregate_pick();
            event.prevent_default();
            return;
        }
        if key == " " || code == "Space" {
            space_down.set(true);
            event.prevent_default();
        } else if key == "Escape" {
            space_down.set(false);
            selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&render_scope));
            let mut interaction_version = interaction_version;
            interaction_version += 1;
        } else if (modifiers.ctrl() || modifiers.meta()) && key.eq_ignore_ascii_case("z") {
            event.prevent_default();
            runtime.submit(if modifiers.shift() {
                Event::Redo {
                    operation_id: runtime.operation(),
                }
            } else {
                Event::Undo {
                    operation_id: runtime.operation(),
                }
            });
        }
    }
}

pub(super) fn key_up_handler(
    navigation: CanvasNavigationState,
) -> impl FnMut(KeyboardEvent) + Clone + 'static {
    move |event: KeyboardEvent| {
        let key = event.data().key().to_string();
        let code = event.data().code().to_string();
        navigation.release_space_key(&key, &code);
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
    pub canvas_interaction: CanvasInteractionArbiter,
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
    let StartPanInput {
        navigation,
        render_scope,
        render_generation,
        runtime,
        svg,
        adapter,
        mirrored_pair,
        canvas_interaction,
        mut pending_splay_pick,
        transform_inspector,
        layout_owner_for_pick,
        workspace,
        view_x,
        view_y,
        width,
        height,
    } = input;
    navigation.space_pan_handler(
        render_scope.clone(),
        render_generation,
        runtime.clone(),
        svg.clone(),
        Rc::new({
            let adapter = adapter.clone();
            move || (adapter.generation)() == render_generation
        }),
        {
            let runtime = runtime.clone();
            let svg = svg.clone();
            let adapter = adapter.clone();
            let mirrored_pair = mirrored_pair.clone();
            let canvas_interaction = canvas_interaction.clone();
            move |event: PointerEvent| {
                let Some(pointer) = event.data().try_as_web_event() else {
                    return;
                };
                if canvas_interaction.is_owner(CanvasInteractionOwner::MatrixPlacement) {
                    pointer.prevent_default();
                    pointer.stop_propagation();
                    return;
                }
                if canvas_interaction.is_owner(CanvasInteractionOwner::MirroredPair)
                    && let Some(placement) = mirrored_pair.placement.as_ref()
                    && pointer.button() == 0
                {
                    let owner = placement.owner.clone();
                    if workspace() != Workspace::Layout
                        || runtime.scope().as_ref() != Some(&owner.scope)
                        || (adapter.generation)() != owner.scope_generation
                    {
                        mirrored_pair.on_cancel.call(owner);
                        return;
                    }
                    if let Some(center) = coordinates(&svg, &pointer, view_x, view_y, width, height)
                    {
                        pointer.prevent_default();
                        pointer.stop_propagation();
                        mirrored_pair
                            .on_commit
                            .call(objects::MirroredPairMove { owner, center });
                    }
                    return;
                }
                if canvas_interaction.current().is_some() {
                    pointer.prevent_default();
                    pointer.stop_propagation();
                    return;
                }
                if pointer.button() == 0
                    && workspace() == Workspace::Layout
                    && pending_splay_pick.peek().is_some()
                    && let Some(point) = coordinates(&svg, &pointer, view_x, view_y, width, height)
                {
                    pointer.prevent_default();
                    pointer.stop_propagation();
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
        },
    )
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
    setup_a: &super::setup_bundles::SetupA,
    cam: &super::camera::CamNamed,
    render_scope: &Scope,
) -> (
    impl FnMut(KeyboardEvent) + Clone + 'static,
    impl FnMut(KeyboardEvent) + Clone + 'static,
    impl FnMut(PointerEvent) + Clone + 'static,
) {
    let runtime = &setup_a.editor.runtime();
    let drag = &cam.placement.drag;
    let svg = &cam.svg_surface.svg;
    let space_down = &cam.placement.space_down;
    let mirrored_pair = &setup_a.matrix_pair.mirrored_pair;
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &setup_a.matrix_pair.matrix_placement;
    let canvas_interaction = &setup_a.canvas_interaction;
    let layout_snap_settings = setup_a.layout_snap_settings;
    let pending_splay_origin_pick = setup_a.pending_splay_origin_pick;
    let pending_anchor_pick = setup_a.pending_anchor_pick;
    let matrix_transform_inspector = &setup_a.matrix_transform_inspector;
    let interaction_version = cam.placement.interaction_version;
    let canvas_navigation = &setup_a.canvas_navigation;
    let adapter = &setup_a.adapter;
    let layout_owner = &setup_a.view.layout_owner;
    let workspace = setup_a.editor.workspace;
    let view_x = cam.scene_bounds.view_x;
    let view_y = cam.scene_bounds.view_y;
    let width = cam.scene_bounds.width;
    let height = cam.scene_bounds.height;
    let render_generation = setup_a.render_generation;
    let keyboard = keyboard_handler(KeyboardInput {
        runtime: runtime.clone(),
        drag: drag.clone(),
        svg: svg.clone(),
        render_scope: render_scope.clone(),
        space_down: space_down.clone(),
        mirrored_pair: mirrored_pair.clone(),
        placement: part_placement.clone(),
        matrix_placement: matrix_placement.clone(),
        canvas_interaction: canvas_interaction.clone(),
        snap_settings: layout_snap_settings,
        pending_splay_pick: pending_splay_origin_pick,
        pending_anchor_pick,
        transform_inspector: matrix_transform_inspector.clone(),
        interaction_version,
    });
    let key_up = key_up_handler(canvas_navigation.clone());
    let start_pan = start_pan_handler(StartPanInput {
        navigation: canvas_navigation.clone(),
        render_scope: render_scope.clone(),
        render_generation,
        runtime: runtime.clone(),
        svg: svg.clone(),
        adapter: adapter.clone(),
        mirrored_pair: mirrored_pair.clone(),
        canvas_interaction: canvas_interaction.clone(),
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
