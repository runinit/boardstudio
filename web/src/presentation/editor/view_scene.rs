use super::super::*;

pub(super) fn polygon_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|p| format!("{},{}", p.x, p.y))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) struct LayoutSvgInput<'a> {
    pub view_box: String,
    pub canvas_aria_label: &'static str,
    pub mount: EventHandler<MountedEvent>,
    pub canvas_grid: &'a objects::LayoutCanvasGridStyle,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
    pub layer_visibility: &'a LayerVisibility,
    pub scene: &'a boardstudio_core::model::SceneDelta,
    pub model: &'a ReadModel,
    pub outline_inspector: &'a Option<outline_lifecycle::OutlineInspectorProjection>,
    pub outline_overlay_key: &'a Option<String>,
    pub runtime: Rc<Runtime>,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub outline_snap_settings: &'a objects::LayoutSnapSettings,
    pub outline_pitch: Vec2,
    pub outline_snap_origins: &'a Rc<Vec<outline_snapping::Origin>>,
    pub selected_bridge: Option<&'a boardstudio_core::model::OutlineBridge>,
    pub matrix_placement: &'a objects::MatrixPlacementMount,
    pub mirrored_pair: &'a objects::MirroredPairMount,
    pub canvas_source: layout_canvas::SceneSource,
    pub split_focused_half: Option<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub spacing_ghost_prop: &'a Option<layout_canvas::SpacingGhost>,
    pub selected_tree_context: &'a Option<objects::ScopedTreeContext>,
    pub zoom_surface_size: Signal<(f64, f64)>,
    pub canvas_anchor_badge: &'a Option<anchor_badge::AnchorBadge>,
    pub matrix_pointer_down: EventHandler<layout_canvas::MatrixPointerDown>,
    pub part_pointer_down: EventHandler<layout_canvas::PartPointerDown>,
    pub selection_geometry_anchor: Option<(f64, f64, f64)>,
    pub canvas_transform_menu: Option<objects::LayoutTransformMenuMount>,
    pub matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub layout_owner: LayoutOwnerIdentity,
    pub adapter: SelectionAdapter,
    pub drag: Rc<RefCell<Option<Drag>>>,
    pub render_scope: Scope,
    pub workspace: Signal<&'static str>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub layout_transform_tool: Signal<Option<objects::LayoutTransformTool>>,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub layout_transform_target: Option<(
        std::rc::Rc<boardstudio_core::model::Matrix>,
        std::rc::Rc<boardstudio_core::model::MatrixScene>,
        objects::TreeContext,
    )>,
    pub matrix_splay_affect: Signal<boardstudio_core::model::MatrixSplayAffect>,
    pub part_rail_selection: Option<(String, boardstudio_web_layout::part_rail::PartRailFacts)>,
    pub part_rail_snap: Option<objects::GestureSnapInputs>,
    pub part_placement: &'a part_placement::PartPlacementMount,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub split_focus: Signal<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub focused_keycaps_finding: Signal<Option<keycaps_finding_marker::FocusedFinding>>,
    pub snapshot: &'a AcceptedSnapshot,
    pub active_workspace: &'static str,
    pub layout_assembly_3d: Signal<bool>,
}

pub(super) struct SvgNavHandlers<S, M, E> {
    pub start_pan: S,
    pub move_pointer: M,
    pub end_pointer: E,
}

pub(super) struct SvgKeyHandlers<C, K, U> {
    pub cancel_pointer: C,
    pub keyboard: K,
    pub key_up: U,
}

pub(super) fn layout_svg<S, M, E, C, K, U>(
    input: LayoutSvgInput<'_>,
    nav: SvgNavHandlers<S, M, E>,
    key: SvgKeyHandlers<C, K, U>,
    wheel: impl FnMut(WheelEvent) + Clone + 'static,
) -> Element
where
    S: FnMut(PointerEvent) + Clone + 'static,
    M: FnMut(PointerEvent) + Clone + 'static,
    E: FnMut(PointerEvent) + Clone + 'static,
    C: FnMut(PointerEvent) + Clone + 'static,
    K: FnMut(KeyboardEvent) + Clone + 'static,
    U: FnMut(KeyboardEvent) + Clone + 'static,
{
    let LayoutSvgInput {
        view_box,
        canvas_aria_label,
        mount,
        canvas_grid,
        view_x,
        view_y,
        width,
        height,
        layer_visibility,
        scene,
        model,
        outline_inspector,
        outline_overlay_key,
        runtime,
        canvas_interaction,
        svg,
        outline_snap_settings,
        outline_pitch,
        outline_snap_origins,
        selected_bridge,
        matrix_placement,
        mirrored_pair,
        canvas_source,
        split_focused_half,
        spacing_ghost_prop,
        selected_tree_context,
        zoom_surface_size,
        canvas_anchor_badge,
        matrix_pointer_down,
        part_pointer_down,
        selection_geometry_anchor,
        canvas_transform_menu,
        matrix_transform_inspector,
        layout_owner,
        adapter,
        drag,
        render_scope,
        workspace,
        layout_state,
        layout_transform_tool,
        layout_snap_settings,
        layout_transform_target,
        matrix_splay_affect,
        part_rail_selection,
        part_rail_snap,
        part_placement,
        document,
        split_focus,
        focused_keycaps_finding,
        snapshot,
        active_workspace,
        layout_assembly_3d,
    } = input;
    let SvgNavHandlers {
        start_pan,
        move_pointer,
        end_pointer,
    } = nav;
    let SvgKeyHandlers {
        cancel_pointer,
        keyboard,
        key_up,
    } = key;
    rsx! {
        svg { class: "mt-[76px] block h-[calc(100%-120px)] min-h-0 w-full touch-none bg-canvas max-[760px]:mt-0 max-[760px]:h-full", view_box: "{view_box}", preserve_aspect_ratio: "xMidYMid meet", tabindex: "0", role: "group", "aria-label": "{canvas_aria_label}", onmounted: mount,
        onpointerdown: start_pan, onpointermove: move_pointer, onpointerup: end_pointer, onpointercancel: cancel_pointer.clone(), onlostpointercapture: cancel_pointer, onkeydown: keyboard, onkeyup: key_up, onwheel: wheel,
        defs {
            pattern { id: "m1-layout-grid-small", width: "{canvas_grid.spacing_mm}", height: "{canvas_grid.spacing_mm}", pattern_units: "userSpaceOnUse",
                circle { cx: "0", cy: "0", r: "{canvas_grid.radius_mm}", fill: "var(--grid-large)", stroke: "none" }
            }
        }
        rect { x: "{view_x}", y: "{view_y}", width: "{width}", height: "{height}", fill: "url(#m1-layout-grid-small)" }
        g { transform: "scale(1,-1)",
            {scene_boards(SceneBoardsInput {
                layer_visibility,
                scene,
                model,
                outline_inspector,
                outline_overlay_key,
                runtime: runtime.clone(),
                canvas_interaction: canvas_interaction.clone(),
                svg: svg.clone(),
                view_x,
                view_y,
                width,
                height,
                outline_snap_settings,
                outline_pitch,
                outline_snap_origins,
                selected_bridge,
            })}
            {scene_placement_previews(layer_visibility, matrix_placement, mirrored_pair)}
            layout_canvas::Scene {
                source: canvas_source,
                focused_half: split_focused_half,
                preview_active: model.display_preview.is_some(),
                accepted_scene: model
                    .display_preview
                    .as_ref()
                    .map(|_| snapshot.scene.clone()),
                spacing_ghost: spacing_ghost_prop.clone(),
                selected_ids: model.selected_part_ids.clone(),
                selected_context: selected_tree_context.clone(),
                view_width_world: width,
                surface_width_px: zoom_surface_size().0,
                anchor_badge: canvas_anchor_badge.clone(),
                keys_visible: !(layer_visibility.hidden)().contains("Keys"),
                components_visible: !(layer_visibility.hidden)().contains("Components"),
                keycaps_visible: !(layer_visibility.hidden)().contains("Keycaps"),
                footprints_visible: (layer_visibility.footprints)()
                    && !(layer_visibility.hidden)().contains("Footprints"),
                on_matrix_pointer_down: matrix_pointer_down,
                on_part_pointer_down: part_pointer_down,
            }
            {selection_anchor(SelectionAnchorInput {
                selection_geometry_anchor,
                canvas_transform_menu,
                matrix_transform_inspector,
                selected_tree_context,
                runtime: runtime.clone(),
                svg: svg.clone(),
                drag,
                layout_owner: layout_owner.clone(),
                adapter: adapter.clone(),
                canvas_interaction: canvas_interaction.clone(),
                render_scope: render_scope.clone(),
                workspace,
                view_x,
                view_y,
                width,
                height,
            })}
            {transform_overlays(TransformOverlaysInput {
                active_workspace,
                layout_assembly_3d,
                layout_transform_target,
                layout_transform_tool,
                layout_state,
                runtime: runtime.clone(),
                svg: svg.clone(),
                canvas_interaction: canvas_interaction.clone(),
                layout_owner: layout_owner.clone(),
                adapter: adapter.clone(),
                zoom_surface_size,
                workspace,
                layout_snap_settings,
                outline_snap_origins,
                matrix_splay_affect,
                view_x,
                view_y,
                width,
                height,
            })}
            {part_rail(PartRailInput {
                active_workspace,
                layout_assembly_3d,
                part_rail_selection,
                selected_tree_context,
                part_rail_snap,
                layout_transform_tool,
                layout_state,
                runtime: runtime.clone(),
                svg: svg.clone(),
                canvas_interaction: canvas_interaction.clone(),
                layout_owner: layout_owner.clone(),
                adapter: adapter.clone(),
                zoom_surface_size,
                workspace,
                document,
                split_focus,
                view_x,
                view_y,
                width,
                height,
            })}
            {placement_preview(part_placement, width, zoom_surface_size)}
            {finding_marker_draft(MarkerDraftInput {
                active_workspace,
                render_scope: render_scope.clone(),
                snapshot,
                model,
                focused_keycaps_finding,
                outline_inspector,
                outline_overlay_key,
                runtime: runtime.clone(),
                canvas_interaction: canvas_interaction.clone(),
                svg: svg.clone(),
                view_x,
                view_y,
                width,
                height,
                outline_snap_settings,
                outline_pitch,
                outline_snap_origins,
            })}
        }
        }
    }
}

pub(super) struct SceneBoardsInput<'a> {
    pub layer_visibility: &'a LayerVisibility,
    pub scene: &'a boardstudio_core::model::SceneDelta,
    pub model: &'a ReadModel,
    pub outline_inspector: &'a Option<outline_lifecycle::OutlineInspectorProjection>,
    pub outline_overlay_key: &'a Option<String>,
    pub runtime: Rc<Runtime>,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
    pub outline_snap_settings: &'a objects::LayoutSnapSettings,
    pub outline_pitch: Vec2,
    pub outline_snap_origins: &'a Rc<Vec<outline_snapping::Origin>>,
    pub selected_bridge: Option<&'a boardstudio_core::model::OutlineBridge>,
}

pub(super) fn scene_boards(input: SceneBoardsInput<'_>) -> Element {
    let SceneBoardsInput {
        layer_visibility,
        scene,
        model,
        outline_inspector,
        outline_overlay_key,
        runtime,
        canvas_interaction,
        svg,
        view_x,
        view_y,
        width,
        height,
        outline_snap_settings,
        outline_pitch,
        outline_snap_origins,
        selected_bridge,
    } = input;
    rsx! {
        if !(layer_visibility.hidden)().contains("Board") {
            for contour in scene.board_contours.iter().filter(|b| b.board_id == model.active_board_id).flat_map(|b| &b.contours) {
                polygon { points: polygon_points(&contour.points), class: if contour.hole { "fill-canvas stroke-destructive stroke-[0.3] [stroke-dasharray:0.8_0.55]" } else { "fill-geometry-fill stroke-geometry-board stroke-[0.3]" } }
            }
        }
        if let Some(projection) = outline_inspector.clone().filter(|projection| (projection.editing_points)()) {
            outline_lifecycle::OutlinePointCanvasOverlay {
                key: "{outline_overlay_key.as_deref().unwrap_or_default()}",
                projection,
                runtime: outline_lifecycle::OutlineRuntimeHandle::new(
                    runtime.clone(),
                ),
                arbiter: canvas_interaction.clone(),
                svg: svg.clone(),
                view_x,
                view_y,
                width,
                height,
                snap_settings: outline_snap_settings.clone(),
                pitch: outline_pitch,
                origins: outline_snap_origins.as_ref().clone(),
            }
        }
        if let Some(bridge) = selected_bridge {
            polygon { points: polygon_points(&bridge.points), class: "pointer-events-none fill-none stroke-primary stroke-[1.25]", "data-outline-bridge": bridge.id.clone() }
        }
    }
}

pub(super) fn scene_placement_previews(
    layer_visibility: &LayerVisibility,
    matrix_placement: &objects::MatrixPlacementMount,
    mirrored_pair: &objects::MirroredPairMount,
) -> Element {
    rsx! {
        if !(layer_visibility.hidden)().contains("Keys") {
            if let Some(active) = matrix_placement.placement.as_ref() {
                {
                    let cells = crate::mirrored_pair_geometry::preview_cells(&active.scene, &active.matrix);
                    rsx! {
                        g {
                            class: "pointer-events-none",
                            "aria-label": "Matrix placement preview",
                            transform: "translate({active.matrix.origin.x} {active.matrix.origin.y})",
                            for cell in cells {
                                g { key: "{cell.row}-{cell.column}", transform: "translate({cell.center.x} {cell.center.y}) rotate({cell.rotation})",
                                    rect { class: "fill-primary/20 stroke-primary stroke-[0.35]", x: "{-cell.size.x / 2.0}", y: "{-cell.size.y / 2.0}", width: "{cell.size.x}", height: "{cell.size.y}", rx: "1" }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(active_pair) = mirrored_pair.placement.as_ref() {
                {
                    let pair = &active_pair.pair;
                    let left_cells = crate::mirrored_pair_geometry::preview_cells(&active_pair.left_scene, &pair.matrix);
                    let right_cells = crate::mirrored_pair_geometry::preview_cells(&active_pair.right_scene, &pair.right_preview);
                    let line_half_height = f64::from(pair.matrix.rows) * pair.matrix.pitch.y / 2.0 + 16.0;
                    rsx! {
                        g { class: "pointer-events-none", "aria-label": "Mirrored pair placement preview", transform: "translate({pair.center.x} {pair.center.y})",
                            line { class: "stroke-primary stroke-[0.35] [stroke-dasharray:2_2]", x1: "0", x2: "0", y1: "{-line_half_height}", y2: "{line_half_height}" }
                            for cell in left_cells {
                                g { key: "left-{cell.row}-{cell.column}", transform: "translate({cell.center.x} {cell.center.y}) rotate({cell.rotation})",
                                    rect { class: "fill-primary/20 stroke-primary stroke-[0.35]", x: "{-cell.size.x / 2.0}", y: "{-cell.size.y / 2.0}", width: "{cell.size.x}", height: "{cell.size.y}", rx: "1" }
                                }
                            }
                            for cell in right_cells {
                                g { key: "right-{cell.row}-{cell.column}", transform: "translate({cell.center.x} {cell.center.y}) rotate({cell.rotation})",
                                    rect { class: "fill-primary/20 stroke-primary stroke-[0.35]", x: "{-cell.size.x / 2.0}", y: "{-cell.size.y / 2.0}", width: "{cell.size.x}", height: "{cell.size.y}", rx: "1" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(super) struct SelectionAnchorInput<'a> {
    pub selection_geometry_anchor: Option<(f64, f64, f64)>,
    pub canvas_transform_menu: Option<objects::LayoutTransformMenuMount>,
    pub matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub selected_tree_context: &'a Option<objects::ScopedTreeContext>,
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub drag: Rc<RefCell<Option<Drag>>>,
    pub layout_owner: LayoutOwnerIdentity,
    pub adapter: SelectionAdapter,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub render_scope: Scope,
    pub workspace: Signal<&'static str>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
}

pub(super) fn selection_anchor(input: SelectionAnchorInput<'_>) -> Element {
    let SelectionAnchorInput {
        selection_geometry_anchor,
        canvas_transform_menu,
        matrix_transform_inspector,
        selected_tree_context,
        runtime,
        svg,
        drag,
        layout_owner,
        adapter,
        canvas_interaction,
        render_scope,
        workspace,
        view_x,
        view_y,
        width,
        height,
    } = input;
    rsx! {
        if let Some((x, y, scale)) = selection_geometry_anchor
            && let Some(tools) = canvas_transform_menu.clone()
            && matrix_transform_inspector.projection.is_some()
        {
            g {
                foreignObject { class: "overflow-visible", transform: "translate({x - 60.0 * scale} {y - 66.0 * scale}) scale({scale} {-scale})", width: "120", height: "44",
                objects::SelectionDeleteControl { mount: matrix_transform_inspector.clone() }
            }
            if !matches!(selected_tree_context.as_ref().map(|selected| &selected.context), Some(objects::TreeContext::Column { .. } | objects::TreeContext::Row { .. })) {
            { let runtime = runtime.clone(); let svg = svg.clone(); let drag = drag.clone();
                  let owner = layout_owner.clone(); let adapter = adapter.clone();
                  let arbiter = canvas_interaction.clone(); let scope = render_scope.clone();
                  rsx! {
                    g {
                        class: "cursor-grab touch-none fill-card stroke-current text-primary outline-none [pointer-events:all] [stroke-width:0.4] active:cursor-grabbing focus-visible:stroke-foreground [&_path]:fill-none",
                        transform: "translate({x} {y - 18.0 * scale}) scale({scale * 3.0})",
                        role: "button", tabindex: "0", "aria-label": "Move selection",
                        "aria-description": "Drag to move. Press Enter to edit the exact position.",
                        onkeydown: move |event: KeyboardEvent| {
                            if event.key() == Key::Enter || event.key() == Key::Character(" ".into()) {
                                event.prevent_default(); event.stop_propagation();
                                tools.on_show_properties.call(());
                            }
                        },
                        onpointerdown: move |event: PointerEvent| {
                            let Some(pointer) = event.data().try_as_web_event() else { return; };
                            if pointer.button() != 0 || arbiter.current().is_some()
                                || !layout_owner_is_current(&runtime, workspace, &adapter, &owner)
                                || drag.borrow().is_some() || runtime.model().gesture.is_some() { return; }
                            pointer.prevent_default(); pointer.stop_propagation();
                            let Some(origin) = coordinates(&svg, &pointer, view_x, view_y, width, height) else { return; };
                            let current = runtime.model();
                            let Some(snapshot) = current.accepted else { return; };
                            let positions: Vec<_> = snapshot.document.parts.iter().filter(|p| current.selected_part_ids.contains(&p.id)).map(|p| Position { id: p.id.clone(), at: p.pose.at }).collect();
                            if positions.is_empty() { return; }
                            if let Some(surface) = svg.borrow().as_ref() {
                                let _ = surface.set_pointer_capture(pointer.pointer_id());
                                let options = web_sys::FocusOptions::new(); options.set_prevent_scroll(true);
                                let _ = surface.focus_with_options(&options);
                            }
                            *drag.borrow_mut() = Some(Drag { pointer: i64::from(pointer.pointer_id()), scope: scope.clone(), generation: owner.generation, gesture_generation: None, origin, client_x: f64::from(pointer.client_x()), client_y: f64::from(pointer.client_y()), positions, active: false, pan: false, camera: Vec2::default() });
                        },
                        circle { r: "5" }
                        path { d: "M0 -3.2v6.4M-3.2 0h6.4M-1.4 -1.8 0 -3.2l1.4 1.4M-1.4 1.8 0 3.2l1.4 -1.4M-1.8 -1.4-3.2 0l1.4 1.4M1.8 -1.4 3.2 0l-1.4 1.4" }
                        title { "Move selection" }
                    }
                  }
                }
            }
            }
        }
    }
}

pub(super) struct TransformOverlaysInput<'a> {
    pub active_workspace: &'static str,
    pub layout_assembly_3d: Signal<bool>,
    pub layout_transform_target: Option<(
        std::rc::Rc<boardstudio_core::model::Matrix>,
        std::rc::Rc<boardstudio_core::model::MatrixScene>,
        objects::TreeContext,
    )>,
    pub layout_transform_tool: Signal<Option<objects::LayoutTransformTool>>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub layout_owner: LayoutOwnerIdentity,
    pub adapter: SelectionAdapter,
    pub zoom_surface_size: Signal<(f64, f64)>,
    pub workspace: Signal<&'static str>,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub outline_snap_origins: &'a Rc<Vec<outline_snapping::Origin>>,
    pub matrix_splay_affect: Signal<boardstudio_core::model::MatrixSplayAffect>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
}

pub(super) fn transform_overlays(input: TransformOverlaysInput<'_>) -> Element {
    let TransformOverlaysInput {
        active_workspace,
        layout_assembly_3d,
        layout_transform_target,
        mut layout_transform_tool,
        layout_state,
        runtime,
        svg,
        canvas_interaction,
        layout_owner,
        adapter,
        zoom_surface_size,
        workspace,
        layout_snap_settings,
        outline_snap_origins,
        matrix_splay_affect,
        view_x,
        view_y,
        width,
        height,
    } = input;
    rsx! {
        if active_workspace == "Layout"
            && !layout_assembly_3d()
            && let Some((matrix, projection, context)) = layout_transform_target.clone()
        {
            for tool in layout_canvas::transform_tools(&context, layout_transform_tool(), layout_state.transform_handles()(), None) {
            // Ticket 08: the anchored matrix's Anchor token
            // arms canvas pick mode instead of a drag; an
            // unanchored matrix offers no token.
            if tool == objects::LayoutTransformTool::Anchor && matrix.anchor.is_some() {
            objects::LayoutAnchorPickOverlay {
                key: "{tool:?}",
                runtime: objects::LayoutTransformRuntime(runtime.clone()),
                svg: objects::LayoutTransformSvg(svg.clone()),
                arbiter: canvas_interaction.clone(),
                owner: layout_owner.clone(),
                selected_context: adapter.selected_context,
                scope_generation: adapter.generation,
                surface_size: Some(zoom_surface_size),
                workspace,
                pick_owner: objects::AnchorPickOwner::Matrix {
                    matrix: matrix.clone(),
                    projection: projection.clone(),
                    context: context.clone(),
                },
                view_x,
                view_y,
                width,
                height,
            }
            }
            if tool != objects::LayoutTransformTool::Anchor {
            objects::LayoutTransformToolOverlay {
                key: "{tool:?}",
                runtime: objects::LayoutTransformRuntime(runtime.clone()),
                svg: objects::LayoutTransformSvg(svg.clone()),
                arbiter: canvas_interaction.clone(),
                owner: layout_owner.clone(),
                selected_context: adapter.selected_context,
                scope_generation: adapter.generation,
                surface_size: Some(zoom_surface_size),
                workspace,
                matrix: matrix.clone(),
                projection: projection.clone(),
                context: context.clone(),
                tool,
                snap_settings: layout_snap_settings.read().clone(),
                snap_origins: outline_snap_origins.clone(),
                splay_affect: matrix_splay_affect,
                view_x,
                view_y,
                width,
                height,
                on_finish: EventHandler::new(move |_| layout_transform_tool.set(None)),
            }
            }
            }
        }
    }
}

pub(super) struct PartRailInput<'a> {
    pub active_workspace: &'static str,
    pub layout_assembly_3d: Signal<bool>,
    pub part_rail_selection: Option<(String, boardstudio_web_layout::part_rail::PartRailFacts)>,
    pub selected_tree_context: &'a Option<objects::ScopedTreeContext>,
    pub part_rail_snap: Option<objects::GestureSnapInputs>,
    pub layout_transform_tool: Signal<Option<objects::LayoutTransformTool>>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub layout_owner: LayoutOwnerIdentity,
    pub adapter: SelectionAdapter,
    pub zoom_surface_size: Signal<(f64, f64)>,
    pub workspace: Signal<&'static str>,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub split_focus: Signal<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
}

pub(super) fn part_rail(input: PartRailInput<'_>) -> Element {
    let PartRailInput {
        active_workspace,
        layout_assembly_3d,
        part_rail_selection,
        selected_tree_context,
        part_rail_snap,
        mut layout_transform_tool,
        layout_state,
        runtime,
        svg,
        canvas_interaction,
        layout_owner,
        adapter,
        zoom_surface_size,
        workspace,
        document,
        split_focus,
        view_x,
        view_y,
        width,
        height,
    } = input;
    rsx! {
        // Ticket 07: the independent part's own rail. Move and
        // Rotate edit the local Offset and Rotation; Spread and
        // Stagger edit the values carried from the anchor key.
        // The strip above is unchanged: it keeps offering
        // Position for a part selection.
        if active_workspace == "Layout"
            && !layout_assembly_3d()
            && let Some((part_id, facts)) = part_rail_selection.clone()
            && let Some(selected) = selected_tree_context.clone()
            && let Some(part_snap) = part_rail_snap
        {
            {
                let part_context = selected.context.clone();
                rsx! {
                    for tool in layout_canvas::transform_tools(&part_context, layout_transform_tool(), layout_state.transform_handles()(), Some(facts)) {
                    // Ticket 08: the part rail ends in the
                    // Anchor token, which arms canvas pick
                    // mode instead of a drag.
                    if tool == objects::LayoutTransformTool::Anchor {
                    objects::LayoutAnchorPickOverlay {
                        key: "{tool:?}",
                        runtime: objects::LayoutTransformRuntime(runtime.clone()),
                        svg: objects::LayoutTransformSvg(svg.clone()),
                        arbiter: canvas_interaction.clone(),
                        owner: layout_owner.clone(),
                        selected_context: adapter.selected_context,
                        scope_generation: adapter.generation,
                        workspace,
                        surface_size: Some(zoom_surface_size),
                        pick_owner: objects::AnchorPickOwner::Part {
                            part_id: part_id.clone(),
                        },
                        view_x,
                        view_y,
                        width,
                        height,
                    }
                    }
                    if tool != objects::LayoutTransformTool::Anchor {
                    objects::LayoutPartToolOverlay {
                        key: "{tool:?}",
                        runtime: objects::LayoutTransformRuntime(runtime.clone()),
                        svg: objects::LayoutTransformSvg(svg.clone()),
                        arbiter: canvas_interaction.clone(),
                        owner: layout_owner.clone(),
                        selected_context: adapter.selected_context,
                        scope_generation: adapter.generation,
                        workspace,
                        surface_size: Some(zoom_surface_size),
                        part_id: part_id.clone(),
                        tool,
                        snap: part_snap,
                        view_x,
                        view_y,
                        width,
                        height,
                        on_finish: EventHandler::new(move |_| layout_transform_tool.set(None)),
                        on_focus_half: {
                            let document = document.clone();
                            let mut split_focus = split_focus;
                            let part_context = part_context.clone();
                            EventHandler::new(move |_| {
                                if let Some(half) = boardstudio_web_ui_model::split_focus::context_half(&document, &part_context)
                                    && half != split_focus()
                                {
                                    split_focus.set(half);
                                }
                            })
                        },
                    }
                    }
                    }
                }
            }
        }
    }
}

pub(super) fn placement_preview(
    part_placement: &part_placement::PartPlacementMount,
    width: f64,
    zoom_surface_size: Signal<(f64, f64)>,
) -> Element {
    rsx! {
        if let Some(placement) = part_placement.projection.as_ref() {
            { let definition = placement.pending.definition.clone();
              let at = placement.pending.part.pose.at;
              let courtyard = polygon_points(&definition.courtyard);
              let reference = placement.pending.part.reference.clone();
              // Ticket 02: the preview name keeps the part-label
              // rule: at the top of the part at a fixed 11 px.
              let preview_courtyard_top = definition.courtyard.iter().map(|point| point.y).fold(f64::NEG_INFINITY, f64::max);
              let preview_courtyard_top = if preview_courtyard_top.is_finite() { preview_courtyard_top } else { 0.0 };
              let preview_label_font = boardstudio_web_layout::screen_scale::px_to_world(11.0, width, zoom_surface_size().0);
              let preview_label_y = preview_courtyard_top - boardstudio_web_layout::screen_scale::px_to_world(12.0, width, zoom_surface_size().0);
              rsx! {
                g {
                    class: "pointer-events-none opacity-[0.68]",
                    "data-placement-preview": "part",
                    transform: "translate({at.x},{at.y})",
                    "aria-hidden": "true",
                    polygon { points: "{courtyard}", class: "fill-selected stroke-primary stroke-[0.7] [stroke-dasharray:2_1]" }
                    FootprintGraphics {
                        definition: definition.clone(),
                        parameters: placement.pending.part.generator_parameters.clone(),
                    }
                    for pad in &definition.pads {
                        g { transform: "translate({pad.at.x} {pad.at.y}) rotate({pad.rotation.unwrap_or(0.0)})",
                            if pad.plated != Some(false) {
                                rect { class: "fill-geometry-part stroke-none", x: "{-pad.size.x / 2.0}", y: "{-pad.size.y / 2.0}", width: "{pad.size.x}", height: "{pad.size.y}" }
                            }
                            if let Some(drill) = pad.drill { circle { class: "fill-canvas stroke-geometry-outline stroke-[0.1]", r: "{drill / 2.0}" } }
                        }
                    }
                    text { transform: "scale(1,-1)", text_anchor: "middle", class: "pointer-events-none fill-geometry-label font-mono", style: "font-size:{preview_label_font}px", x: "0", y: "{preview_label_y}", "{reference}" }
                }
              }
            }
        }
    }
}

pub(super) struct MarkerDraftInput<'a> {
    pub active_workspace: &'static str,
    pub render_scope: Scope,
    pub snapshot: &'a AcceptedSnapshot,
    pub model: &'a ReadModel,
    pub focused_keycaps_finding: Signal<Option<keycaps_finding_marker::FocusedFinding>>,
    pub outline_inspector: &'a Option<outline_lifecycle::OutlineInspectorProjection>,
    pub outline_overlay_key: &'a Option<String>,
    pub runtime: Rc<Runtime>,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
    pub outline_snap_settings: &'a objects::LayoutSnapSettings,
    pub outline_pitch: Vec2,
    pub outline_snap_origins: &'a Rc<Vec<outline_snapping::Origin>>,
}

pub(super) fn finding_marker_draft(input: MarkerDraftInput<'_>) -> Element {
    let MarkerDraftInput {
        active_workspace,
        render_scope,
        snapshot,
        model,
        focused_keycaps_finding,
        outline_inspector,
        outline_overlay_key,
        runtime,
        canvas_interaction,
        svg,
        view_x,
        view_y,
        width,
        height,
        outline_snap_settings,
        outline_pitch,
        outline_snap_origins,
    } = input;
    rsx! {
        keycaps_finding_marker::FocusedFindingMarker {
            workspace: active_workspace.to_owned(),
            scope: Some(render_scope.clone()),
            token: Some(snapshot.token),
            revision: Some(snapshot.document.revision),
            active_board_id: model.active_board_id.clone(),
            finding: focused_keycaps_finding(),
            markers: Rc::from(snapshot.scene.finding_markers.clone()),
        }
        if let Some(projection) = outline_inspector.clone().filter(|projection| (projection.drawing_operation)().is_some()) {
            outline_lifecycle::OutlineDraftCanvasOverlay {
                key: "{outline_overlay_key.as_deref().unwrap_or_default()}-draft",
                projection,
                runtime: outline_lifecycle::OutlineRuntimeHandle::new(runtime.clone()),
                arbiter: canvas_interaction.clone(),
                svg: svg.clone(),
                view_x,
                view_y,
                width,
                height,
                snap_settings: outline_snap_settings.clone(),
                pitch: outline_pitch,
                origins: outline_snap_origins.as_ref().clone(),
            }
        }
    }
}
