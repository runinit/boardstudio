use super::super::*;
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

pub(super) struct OnPcbPartPointerDownInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub drag: Rc<RefCell<Option<Drag>>>,
    pub owner: LayoutOwnerIdentity,
    pub scope: Scope,
    pub accepted_token: boardstudio_application::SnapshotToken,
    pub accepted_revision: u64,
    pub generation: u64,
    pub navigation: CanvasNavigationState,
    pub space_down: Rc<Cell<bool>>,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub workspace: Signal<Workspace>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
    pub instance_selection: InstanceSelection,
    pub layout_selection_kind: Signal<objects::LayoutSelectionKind>,
}

pub(super) fn on_pcb_part_pointer_down_handler(
    input: OnPcbPartPointerDownInput,
) -> EventHandler<pcb_scene::PcbPartPointerDown> {
    let OnPcbPartPointerDownInput {
        runtime,
        adapter,
        svg,
        drag,
        owner,
        scope,
        accepted_token,
        accepted_revision,
        generation,
        navigation,
        space_down,
        tree_cell_anchor,
        canvas_interaction,
        workspace,
        view_x,
        view_y,
        width,
        height,
        instance_selection,
        layout_selection_kind,
    } = input;
    EventHandler::new(move |request: pcb_scene::PcbPartPointerDown| {
        if request.scope != scope
            || request.token != accepted_token
            || request.generation != generation
            || !pcb_owner_is_current(&runtime, workspace, &adapter, &owner)
            || !instance_selection.is_current(&runtime.model())
            || canvas_interaction.current().is_some()
            || drag.borrow().is_some()
            || runtime.model().gesture.is_some()
        {
            return;
        }
        let Some(origin) = coordinates_at(
            &svg,
            request.client_x,
            request.client_y,
            view_x,
            view_y,
            width,
            height,
        ) else {
            return;
        };
        if space_down.get() {
            if let Some(surface) = svg.borrow().as_ref() {
                let _ = surface.set_pointer_capture(request.pointer_id as i32);
                let options = web_sys::FocusOptions::new();
                options.set_prevent_scroll(true);
                let _ = surface.focus_with_options(&options);
            }
            navigation.start_space_pan(boardstudio_web_ui_shared::canvas_navigation::PanStart {
                pointer_id: request.pointer_id,
                scope: scope.clone(),
                generation,
                client: (f64::from(request.client_x), f64::from(request.client_y)),
                camera: runtime.model().camera.center,
            });
            return;
        }
        let model = runtime.model();
        let Some(board) = model.accepted.as_ref().and_then(|snapshot| {
            snapshot
                .document
                .boards
                .iter()
                .find(|board| board.id == scope.board_id)
        }) else {
            return;
        };
        if !board.part_ids.contains(&request.part_id) {
            return;
        }
        let Some(hit_context) = objects::context_for_part(&model, &request.part_id) else {
            return;
        };
        if !selection::context_is_current(&model, &scope, &hit_context) {
            return;
        }
        update_tree_cell_anchor(&tree_cell_anchor, &owner, &model, Some(&hit_context));
        let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
        let projection = objects::context_for_selection_kind(
            &model,
            &hit_context,
            layout_selection_kind(),
            retained.as_ref(),
        );
        let context = projection
            .as_ref()
            .map(|projection| projection.context.clone())
            .unwrap_or(hit_context);
        let mode = if request.range {
            SelectionMode::Range
        } else if request.additive {
            SelectionMode::Toggle
        } else {
            SelectionMode::Replace
        };
        let range_ids = if mode == SelectionMode::Range {
            selection::eligible_live_ids(&model)
                .into_iter()
                .filter(|id| board.part_ids.contains(id))
                .collect()
        } else {
            Vec::new()
        };
        if !model.selected_part_ids.contains(&request.part_id)
            || mode != SelectionMode::Replace
            || projection
                .as_ref()
                .is_some_and(|projection| projection.part_ids != model.selected_part_ids)
        {
            selection::submit_canvas_selection(
                &runtime,
                &adapter,
                &scope,
                generation,
                context.clone(),
                mode,
                range_ids,
            );
        } else {
            let mut selected_context = adapter.selected_context;
            selected_context.set(Some(objects::ScopedTreeContext {
                scope: scope.clone(),
                context: context.clone(),
            }));
        }
        // Modified hits change selection only, matching the reference.
        if mode != SelectionMode::Replace {
            return;
        }
        if runtime.scope().as_ref() != Some(&scope) || (adapter.generation)() != generation {
            return;
        }
        let current = runtime.model();
        let Some(current_snapshot) = current.accepted.as_ref() else {
            return;
        };
        if current_snapshot.token != request.token
            || current_snapshot.document.revision != accepted_revision
            || !current.selected_part_ids.contains(&request.part_id)
        {
            return;
        }
        let positions: Vec<_> = current_snapshot
            .document
            .parts
            .iter()
            .filter(|part| {
                current.selected_part_ids.contains(&part.id)
                    && board.part_ids.contains(&part.id)
                    && part.locked != Some(true)
            })
            .map(|part| Position {
                id: part.id.clone(),
                at: part.pose.at,
            })
            .collect();
        if positions.is_empty()
            || positions.len() != current.selected_part_ids.len()
            || current.lifecycle != Lifecycle::Ready
            || current.durability
                != (Durability::Saved {
                    revision: current_snapshot.document.revision,
                })
            || current.display_preview.is_some()
            || current.gesture.is_some()
        {
            return;
        }
        if let Some(surface) = svg.borrow().as_ref() {
            let _ = surface.set_pointer_capture(request.pointer_id as i32);
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = surface.focus_with_options(&options);
        }
        *drag.borrow_mut() = Some(Drag {
            pointer: request.pointer_id,
            scope: scope.clone(),
            generation,
            gesture_generation: None,
            origin,
            client_x: f64::from(request.client_x),
            client_y: f64::from(request.client_y),
            positions,
            active: false,
            pan: false,
            camera: Vec2::default(),
        });
    })
}

pub(super) struct OnPcbEmptyHitInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub scope: Scope,
    pub generation: u64,
    pub token: boardstudio_application::SnapshotToken,
    pub space_down: Rc<Cell<bool>>,
    pub drag: Rc<RefCell<Option<Drag>>>,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub workspace: Signal<Workspace>,
    pub instance_selection: InstanceSelection,
}

pub(super) fn on_pcb_empty_hit_handler(input: OnPcbEmptyHitInput) -> EventHandler<PointerEvent> {
    let OnPcbEmptyHitInput {
        runtime,
        adapter,
        scope,
        generation,
        token,
        space_down,
        drag,
        canvas_interaction,
        workspace,
        instance_selection,
    } = input;
    EventHandler::new(move |event: PointerEvent| {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        if pointer.button() != 0
            || space_down.get()
            || drag.borrow().is_some()
            || canvas_interaction.current().is_some()
            || workspace() != Workspace::Pcb
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let model = runtime.model();
        if !active_board_scope_matches(&model, &scope) || !instance_selection.is_current(&model) {
            return;
        }
        let Some(snapshot) = model
            .accepted
            .as_ref()
            .filter(|snapshot| snapshot.token == token)
        else {
            return;
        };
        if !snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
        {
            return;
        }
        let mut selected_context = adapter.selected_context;
        selected_context.set(None);
        let mut anchor_scope = adapter.anchor_scope;
        anchor_scope.set(None);
        runtime.submit(Event::SelectParts {
            operation_id: runtime.operation(),
            part_ids: Vec::new(),
            range_part_ids: Vec::new(),
            mode: SelectionMode::Replace,
        });
    })
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

pub(super) struct PkNamed {
    pub on_pcb_part_pointer_down: EventHandler<pcb_scene::PcbPartPointerDown>,
    pub on_pcb_empty_hit: EventHandler<PointerEvent>,
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

pub(super) fn build_keyboard_rest(
    setup_a: &super::setup_bundles::SetupA,
    cam: &super::camera::CamNamed,
    snapshot: &AcceptedSnapshot,
    render_scope: &Scope,
) -> (PkNamed, impl FnMut(WheelEvent) + Clone + 'static) {
    let runtime = &setup_a.editor.runtime();
    let drag = &cam.placement.drag;
    let svg = &cam.svg_surface.svg;
    let space_down = &cam.placement.space_down;
    let canvas_interaction = &setup_a.canvas_interaction;
    let canvas_navigation = &setup_a.canvas_navigation;
    let adapter = &setup_a.adapter;
    let layout_owner = &setup_a.view.layout_owner;
    let workspace = setup_a.editor.workspace;
    let view_x = cam.scene_bounds.view_x;
    let view_y = cam.scene_bounds.view_y;
    let width = cam.scene_bounds.width;
    let height = cam.scene_bounds.height;
    let render_generation = setup_a.render_generation;
    let instance_selection = setup_a.editor.instance_selection;
    let layout_selection_kind = setup_a.layout_selection_kind;
    let keymap_scale_ratio = cam.scene_bounds.keymap_scale_ratio;
    let min_x = cam.scene_bounds.min_x;
    let max_x = cam.scene_bounds.max_x;
    let min_y = cam.scene_bounds.min_y;
    let max_y = cam.scene_bounds.max_y;
    let tree_cell_anchor = &setup_a.tree_cell_anchor;
    let on_pcb_part_pointer_down = on_pcb_part_pointer_down_handler(OnPcbPartPointerDownInput {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        svg: svg.clone(),
        drag: drag.clone(),
        owner: layout_owner.clone(),
        scope: render_scope.clone(),
        accepted_token: snapshot.token,
        accepted_revision: snapshot.document.revision,
        generation: render_generation,
        navigation: canvas_navigation.clone(),
        space_down: space_down.clone(),
        tree_cell_anchor: tree_cell_anchor.clone(),
        canvas_interaction: canvas_interaction.clone(),
        workspace,
        view_x,
        view_y,
        width,
        height,
        instance_selection,
        layout_selection_kind,
    });
    let on_pcb_empty_hit = on_pcb_empty_hit_handler(OnPcbEmptyHitInput {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        scope: render_scope.clone(),
        generation: render_generation,
        token: snapshot.token,
        space_down: space_down.clone(),
        drag: drag.clone(),
        canvas_interaction: canvas_interaction.clone(),
        workspace,
        instance_selection,
    });
    let wheel = wheel_handler(WheelInput {
        runtime: runtime.clone(),
        svg: svg.clone(),
        scope: render_scope.clone(),
        adapter: adapter.clone(),
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
    });
    (
        PkNamed {
            on_pcb_part_pointer_down,
            on_pcb_empty_hit,
        },
        wheel,
    )
}
