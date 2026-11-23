use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(in crate::presentation) fn polygon_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|p| format!("{},{}", p.x, p.y))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(in crate::presentation) struct LayoutCanvasExtras {
    pub bridge: Option<boardstudio_core::model::OutlineBridge>,
    pub focused_half: Option<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub anchor: Option<(f64, f64, f64)>,
}

pub(in crate::presentation) fn use_layout_canvas(
    frame: &workspace_composition::WorkspaceFrame,
    canvas: &super::inspector_wiring::CanvasNamed,
    transform_menu: &objects::LayoutTransformMenuMount,
    pointer: &super::pointer::LayoutPointer,
    overlay: &OutlineOverlay,
    extras: LayoutCanvasExtras,
) -> Element {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let layout_state = use_context::<layout_workspace::LayoutWorkspaceState>();
    let interaction = use_context::<workspace_composition::WorkspaceInteraction>();
    let keycaps = use_context::<keycaps_workspace::KeycapsWorkspaceState>();
    let adapter = use_context::<SelectionAdapter>();
    let layout_assembly_3d = layout_state.assembly_3d();
    let focused_keycaps_finding = keycaps.focused_finding();
    if layout_assembly_3d() {
        return rsx! {
            layout_viewer::LayoutCanonicalViewer {
                on_mounted_module_pick: Some(frame.select_module),
                focused_finding: focused_keycaps_finding(),
            }
        };
    }
    let services = &frame.layout_services;
    let LayoutCanvasExtras {
        bridge,
        focused_half,
        anchor,
    } = extras;
    let canvas_aria_label = canvas.inspector.canvas_aria_label;
    let canvas_grid = &overlay.canvas_grid;
    let outline_inspector = &services.outline_inspector;
    let outline_overlay_key = &overlay.outline_overlay_key;
    let outline_snap_settings = &overlay.outline_snap_settings;
    let outline_pitch = overlay.outline_pitch;
    let outline_snap_origins = &overlay.outline_snap_origins;
    let selected_bridge = bridge.as_ref();
    let matrix_placement = &services.matrix_placement;
    let mirrored_pair = &services.mirrored_pair;
    let split_focused_half = focused_half;
    let spacing_ghost_prop = canvas.selection.spacing_ghost_prop.clone();
    let selected_tree_context = &canvas.selection.selected_tree_context;
    let canvas_anchor_badge = canvas.selection.canvas_anchor_badge.clone();
    let matrix_pointer_down = pointer.matrix_pointer_down;
    let part_pointer_down = pointer.part_pointer_down;
    let selection_geometry_anchor = anchor;
    let canvas_transform_menu = Some(transform_menu.clone());
    let matrix_transform_inspector = services.matrix_transform_inspector.clone();
    let drag = interaction.navigation.drag_interaction();
    let layout_transform_target = canvas.selection.layout_transform_target.clone();
    let part_rail_selection = canvas.selection.part_rail_selection.clone();
    let part_rail_snap = canvas.selection.part_rail_snap;
    let workspace_composition::CanvasEventHandlers {
        mount,
        start_pan,
        move_pointer,
        end_pointer,
        cancel_pointer,
        keyboard,
        key_up,
        wheel,
    } = frame.handlers;
    let camera = &frame.camera;
    let view_box = &camera.view_box;
    let (view_x, view_y, width, height) =
        (camera.view_x, camera.view_y, camera.width, camera.height);
    let layer_visibility = &editor.layers;
    let scene = frame.scene.as_ref();
    let model = frame.model.as_ref();
    let runtime = editor.runtime();
    let canvas_interaction = interaction.arbiter;
    let svg = &frame.svg;
    let canvas_source = camera.canvas_source.clone();
    let zoom_surface_size = frame.surface_size;
    let layout_owner = &frame.owner;
    let render_scope = &frame.scope;
    let workspace = editor.workspace;
    let layout_transform_tool = layout_state.transform_tool();
    let layout_snap_settings = layout_state.snap_settings();
    let matrix_splay_affect = layout_state.splay_affect();
    let part_placement = &frame.part_placement;
    let document = &frame.document;
    let split_focus = layout_state.split_focus();
    let snapshot = &frame.snapshot;
    let active_workspace = Workspace::Layout;
    rsx! {
        svg { class: "mt-[76px] block h-[calc(100%-120px)] min-h-0 w-full touch-none bg-canvas max-[760px]:mt-0 max-[760px]:h-full", view_box: "{view_box}", preserve_aspect_ratio: "xMidYMid meet", tabindex: "0", role: "group", "aria-label": "{canvas_aria_label}", onmounted: mount,
        onpointerdown: start_pan, onpointermove: move_pointer, onpointerup: end_pointer, onpointercancel: cancel_pointer, onlostpointercapture: cancel_pointer, onkeydown: keyboard, onkeyup: key_up, onwheel: wheel,
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
                canvas_transform_menu: canvas_transform_menu.clone(),
                matrix_transform_inspector: matrix_transform_inspector.clone(),
                selected_tree_context,
                runtime: runtime.clone(),
                svg: svg.clone(),
                drag: drag.clone(),
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
                layout_transform_target: layout_transform_target.clone(),
                layout_transform_tool,
                layout_state,
                runtime: runtime.clone(),
                svg: svg.clone(),
                canvas_interaction: canvas_interaction.clone(),
                zoom_surface_size,
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
                part_rail_selection: part_rail_selection.clone(),
                selected_tree_context,
                part_rail_snap,
                layout_transform_tool,
                layout_state,
                runtime: runtime.clone(),
                svg: svg.clone(),
                canvas_interaction: canvas_interaction.clone(),
                zoom_surface_size,
                document: document.clone(),
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

pub(in crate::presentation) struct SceneBoardsInput<'a> {
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

pub(in crate::presentation) fn scene_boards(input: SceneBoardsInput<'_>) -> Element {
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

pub(in crate::presentation) fn scene_placement_previews(
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

pub(in crate::presentation) struct SelectionAnchorInput<'a> {
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
    pub workspace: Signal<Workspace>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
}

pub(in crate::presentation) fn selection_anchor(input: SelectionAnchorInput<'_>) -> Element {
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
            && matrix_transform_inspector.is_active()
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

pub(in crate::presentation) struct TransformOverlaysInput<'a> {
    pub active_workspace: Workspace,
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
    pub zoom_surface_size: Signal<(f64, f64)>,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub outline_snap_origins: &'a Rc<Vec<outline_snapping::Origin>>,
    pub matrix_splay_affect: Signal<boardstudio_core::model::MatrixSplayAffect>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
}

pub(in crate::presentation) fn transform_overlays(input: TransformOverlaysInput<'_>) -> Element {
    let TransformOverlaysInput {
        active_workspace,
        layout_assembly_3d,
        layout_transform_target,
        mut layout_transform_tool,
        layout_state,
        runtime,
        svg,
        canvas_interaction,
        zoom_surface_size,
        layout_snap_settings,
        outline_snap_origins,
        matrix_splay_affect,
        view_x,
        view_y,
        width,
        height,
    } = input;
    rsx! {
        if active_workspace == Workspace::Layout
            && !layout_assembly_3d()
            && let Some((matrix, projection, context)) = layout_transform_target.clone()
        {
            {
                // The matrix rail ends in the Anchor token only while the matrix is anchored.
                let tools: Vec<_> = objects::transform_tools(&context, layout_transform_tool(), layout_state.transform_handles()(), None)
                    .into_iter()
                    .filter(|tool| *tool != objects::LayoutTransformTool::Anchor || matrix.anchor.is_some())
                    .collect();
                let anchored = tools.contains(&objects::LayoutTransformTool::Anchor);
                let slots = tools.len();
                rsx! {
                    objects::LayoutRailOverlay {
                        runtime: objects::LayoutTransformRuntime(runtime.clone()),
                        surface: svg.clone(),
                        arbiter: canvas_interaction.clone(),
                        target: objects::RailTarget::Matrix { matrix: matrix.clone(), projection: projection.clone(), context: context.clone() },
                        tools,
                        snap_settings: layout_snap_settings.read().clone(),
                        snap_origins: outline_snap_origins.clone(),
                        splay_affect: matrix_splay_affect,
                        surface_size: Some(zoom_surface_size),
                        view_x, view_y, width, height,
                        on_finish: EventHandler::new(move |_| layout_transform_tool.set(None)),
                    }
                    if anchored {
                        objects::LayoutAnchorPickOverlay {
                            runtime: objects::LayoutTransformRuntime(runtime.clone()),
                            surface: svg.clone(),
                            arbiter: canvas_interaction.clone(),
                            pick_owner: objects::AnchorPickOwner::Matrix { matrix: matrix.clone(), projection: projection.clone(), context: context.clone() },
                            rail_slots: slots,
                            surface_size: Some(zoom_surface_size),
                            view_x, view_y, width, height,
                        }
                    }
                }
            }
        }
    }
}

pub(in crate::presentation) struct PartRailInput<'a> {
    pub active_workspace: Workspace,
    pub layout_assembly_3d: Signal<bool>,
    pub part_rail_selection: Option<(String, boardstudio_web_layout::part_rail::PartRailFacts)>,
    pub selected_tree_context: &'a Option<objects::ScopedTreeContext>,
    pub part_rail_snap: Option<objects::GestureSnapInputs>,
    pub layout_transform_tool: Signal<Option<objects::LayoutTransformTool>>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub zoom_surface_size: Signal<(f64, f64)>,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub split_focus: Signal<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub view_x: f64,
    pub view_y: f64,
    pub width: f64,
    pub height: f64,
}

pub(in crate::presentation) fn part_rail(input: PartRailInput<'_>) -> Element {
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
        zoom_surface_size,
        document,
        split_focus,
        view_x,
        view_y,
        width,
        height,
    } = input;
    rsx! {
        // The independent part's own rail: Move and Rotate edit the local Offset and Rotation (or
        // the pose, while detached); Spread and Stagger edit the values carried from the anchor key.
        if active_workspace == Workspace::Layout
            && !layout_assembly_3d()
            && let Some((part_id, facts)) = part_rail_selection.clone()
            && let Some(selected) = selected_tree_context.clone()
            && let Some(part_snap) = part_rail_snap
        {
            {
                let part_context = selected.context.clone();
                let tools = objects::transform_tools(&part_context, layout_transform_tool(), layout_state.transform_handles()(), Some(facts));
                let slots = tools.len();
                let anchored = tools.contains(&objects::LayoutTransformTool::Anchor);
                let on_focus_half = {
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
                };
                rsx! {
                    objects::LayoutRailOverlay {
                        runtime: objects::LayoutTransformRuntime(runtime.clone()),
                        surface: svg.clone(),
                        arbiter: canvas_interaction.clone(),
                        target: objects::RailTarget::Part { part_id: part_id.clone(), snap: part_snap, on_focus_half },
                        tools,
                        snap_settings: Default::default(),
                        snap_origins: std::rc::Rc::new(Vec::new()),
                        splay_affect: layout_state.splay_affect(),
                        surface_size: Some(zoom_surface_size),
                        view_x, view_y, width, height,
                        on_finish: EventHandler::new(move |_| layout_transform_tool.set(None)),
                    }
                    if anchored {
                        objects::LayoutAnchorPickOverlay {
                            runtime: objects::LayoutTransformRuntime(runtime.clone()),
                            surface: svg.clone(),
                            arbiter: canvas_interaction.clone(),
                            pick_owner: objects::AnchorPickOwner::Part { part_id: part_id.clone() },
                            rail_slots: slots,
                            surface_size: Some(zoom_surface_size),
                            view_x, view_y, width, height,
                        }
                    }
                }
            }
        }
    }
}

pub(in crate::presentation) fn placement_preview(
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

pub(in crate::presentation) struct MarkerDraftInput<'a> {
    pub active_workspace: Workspace,
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

pub(in crate::presentation) fn finding_marker_draft(input: MarkerDraftInput<'_>) -> Element {
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
            workspace: active_workspace,
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

// --- Canvas selection state (spacing ghost, transform target, part rail) ---

pub(in crate::presentation) struct CanvasSelectionState {
    pub selected_tree_context: Option<objects::ScopedTreeContext>,
    pub canvas_anchor_badge: Option<anchor_badge::AnchorBadge>,
    pub spacing_ghost_prop: Option<layout_canvas::SpacingGhost>,
    pub layout_transform_target: Option<(
        std::rc::Rc<boardstudio_core::model::Matrix>,
        std::rc::Rc<boardstudio_core::model::MatrixScene>,
        objects::TreeContext,
    )>,
    pub part_rail_selection: Option<(String, boardstudio_web_layout::part_rail::PartRailFacts)>,
    pub part_rail_snap: Option<objects::GestureSnapInputs>,
}

pub(in crate::presentation) fn use_canvas_selection_state(
    frame: &workspace_composition::WorkspaceFrame,
) -> CanvasSelectionState {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let adapter = use_context::<SelectionAdapter>();
    let layout_state = use_context::<layout_workspace::LayoutWorkspaceState>();
    let version = editor.version;
    let runtime = editor.runtime();
    // A spacing commit shows the previous accepted scene until another edit,
    // undo/redo, or selection change. The tracker only writes view state.
    let mut last_spacing_seen = use_signal(|| None::<spacing_ghost::SeenAccepted>);
    let mut spacing_ghost_state = use_signal(|| None::<spacing_ghost::SpacingGhostState>);
    let ghost_effect_runtime = runtime.clone();
    let ghost_effect_selection = adapter.selected_context;
    use_effect(move || {
        let _ = version();
        let tree_context = (ghost_effect_selection)();
        let model = ghost_effect_runtime.model();
        let Some(snapshot) = model.accepted.clone() else {
            return;
        };
        let current = spacing_ghost::AcceptedView {
            revision: snapshot.document.revision,
            transaction: snapshot.scene.transaction_id.clone(),
            document: snapshot.document.clone(),
            scene: snapshot.scene.clone(),
            selected_ids: model.selected_part_ids.clone(),
            selected_context: tree_context,
        };
        // Clone the settled values first: the tracker borrows nothing, and the
        // guarded writes below only fire on real advances.
        let seen_before = last_spacing_seen.peek().clone();
        let ghost_before = spacing_ghost_state.peek().clone();
        match spacing_ghost::advance_spacing_ghost(
            seen_before.as_ref(),
            &current,
            ghost_before.as_ref(),
        ) {
            spacing_ghost::GhostAdvance::Keep => {}
            spacing_ghost::GhostAdvance::Set { seen, ghost } => {
                if Some(&seen) != seen_before.as_ref() {
                    last_spacing_seen.set(Some(seen));
                }
                if ghost != ghost_before {
                    spacing_ghost_state.set(ghost);
                }
            }
        }
    });
    let render_scope = frame.scope.clone();
    let model = &frame.model;
    let snapshot = &frame.snapshot;
    let layer_visibility = editor.layers;
    let canvas_source = frame.camera.canvas_source.clone();
    let accepted_canvas_source = frame.camera.accepted_canvas_source.clone();
    let document = frame.document.clone();
    let active_workspace = Workspace::Layout;
    let layout_snap_settings = layout_state.snap_settings();
    let layout_owner = frame.owner.clone();
    let layout_assembly_3d = layout_state.assembly_3d();
    let selected_tree_context = adapter.selected_context.read().clone().filter(|selected| {
        selected.scope == render_scope
            && selection::context_is_current(model, &selected.scope, &selected.context)
    });
    // Ticket 02: the anchor badge and the "Placed from" readout share one
    // projection of the displayed scene. It clears with the selection, and
    // the canvas ring reads the same value through its scene prop.
    let canvas_anchor_badge = (!(layer_visibility.hidden)().contains("Keys"))
        .then(|| {
            selected_tree_context.as_ref().and_then(|selected| {
                let scene_refs: Vec<_> = canvas_source.scene.matrix_scenes.iter().collect();
                anchor_badge::anchor_badge(
                    &canvas_source.document,
                    &scene_refs,
                    Some(&selected.context),
                )
            })
        })
        .flatten();
    // Ticket 18: the live spacing ghost, if it still describes this paint —
    // the revision and selection it was born with, and no drag preview up.
    // The revision check covers the paint between a commit landing and the
    // tracker effect recording it; the selection check covers the paint
    // between a selection landing and the effect retiring the ghost.
    let spacing_ghost_prop = spacing_ghost_state
        .read()
        .clone()
        .filter(|state| {
            state.revision == snapshot.document.revision
                && state.selected_ids == model.selected_part_ids
                && state.selected_context == selected_tree_context
                && model.display_preview.is_none()
        })
        .map(|state| layout_canvas::SpacingGhost {
            scene: state.scene.clone(),
            delta_mm: state.delta_mm,
            axis_x: state.axis_x,
        });
    let transform_matrix_id = selected_tree_context
        .as_ref()
        .filter(|selected| selected.scope == render_scope && active_workspace == Workspace::Layout)
        .and_then(|selected| {
            let matrix_id = match &selected.context {
                objects::TreeContext::Matrix { matrix_id }
                | objects::TreeContext::Row { matrix_id, .. }
                | objects::TreeContext::Column { matrix_id, .. }
                | objects::TreeContext::Key { matrix_id, .. } => matrix_id.as_str(),
                objects::TreeContext::Component {
                    matrix_id: Some(matrix_id),
                    ..
                } => matrix_id.as_str(),
                _ => return None,
            };
            Some(matrix_id.to_owned())
        });
    let transform_matrix = use_memo(use_reactive(
        (&accepted_canvas_source, &transform_matrix_id),
        |(source, matrix_id)| {
            let matrix_id = matrix_id?;
            source
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .cloned()
                .map(Rc::new)
        },
    ))();
    let transform_projection = use_memo(use_reactive(
        (&canvas_source, &transform_matrix_id),
        |(source, matrix_id)| {
            let matrix_id = matrix_id?;
            source
                .scene
                .matrix_scenes
                .iter()
                .find(|matrix| matrix.matrix_id == matrix_id)
                .cloned()
                .map(Rc::new)
        },
    ))();
    let layout_transform_target = transform_matrix
        .zip(transform_projection)
        .zip(selected_tree_context.as_ref())
        .map(|((matrix, projection), selected)| (matrix, projection, selected.context.clone()));
    // Ticket 07: the part rail facts for an independent part selection behind
    // `transform_tools` and the part rail below. Members and companions keep
    // their matrix's tools: the facts name them and the pure list stays empty.
    let part_rail_selection =
        selected_tree_context
            .as_ref()
            .and_then(|selected| match &selected.context {
                objects::TreeContext::Component {
                    part_id: Some(part_id),
                    matrix_id: None,
                    ..
                } => {
                    let part = document.parts.iter().find(|part| &part.id == part_id)?;
                    Some((
                        part_id.clone(),
                        boardstudio_web_layout::part_rail::PartRailFacts {
                            anchored: part.anchor.is_some(),
                            anchor_column: part.anchor.as_ref().map_or(0, |anchor| anchor.column),
                            locked: part.locked == Some(true),
                            member_or_companion: document
                                .matrices
                                .iter()
                                .any(|matrix| matrix.part_ids.contains(part_id)),
                        },
                    ))
                }
                _ => None,
            });
    // Ticket 07: the part rail drag snap: anchored drags snap in the anchor
    // matrix's pitch, like the matrix tools snap in their own.
    let part_rail_snap = part_rail_selection.as_ref().map(|(part_id, _)| {
        let anchor_pitch = document
            .parts
            .iter()
            .find(|part| &part.id == part_id)
            .and_then(|part| part.anchor.as_ref())
            .and_then(|anchor| {
                document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == anchor.matrix_id)
            })
            .map(|matrix| matrix.pitch);
        objects::gesture_snap_inputs(&layout_snap_settings.read().clone(), anchor_pitch, None)
    });
    let transform_tool_owner = (
        layout_owner.scope.clone(),
        layout_owner.generation,
        layout_transform_target
            .as_ref()
            .map(|(matrix, _, _)| matrix.id.clone()),
        active_workspace,
        layout_assembly_3d(),
    );
    layout_state.observe_transform_tool_owner(transform_tool_owner);
    CanvasSelectionState {
        selected_tree_context,
        canvas_anchor_badge,
        spacing_ghost_prop,
        layout_transform_target,
        part_rail_selection,
        part_rail_snap,
    }
}

/// Only a linked Layout split has an unfocused half.
pub(in crate::presentation) fn split_focused_half(
    document: &boardstudio_core::model::ProjectDoc,
    split_focus: Signal<boardstudio_web_ui_model::split_focus::SplitHalf>,
) -> Option<boardstudio_web_ui_model::split_focus::SplitHalf> {
    if boardstudio_web_ui_model::split_focus::has_split(document) {
        Some(split_focus())
    } else {
        None
    }
}

/// The outline bridge named by the selected tree context, for the Layout
/// canvas overlay.
pub(in crate::presentation) fn selected_bridge(
    snapshot: &AcceptedSnapshot,
    selected: Option<&objects::ScopedTreeContext>,
) -> Option<boardstudio_core::model::OutlineBridge> {
    let objects::TreeContext::Bridge {
        board_id,
        bridge_id,
    } = &selected?.context
    else {
        return None;
    };
    snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == *board_id)
        .and_then(|scene| scene.bridges.iter().find(|bridge| bridge.id == *bridge_id))
        .cloned()
}

/// Outline pitch/snap/grid derivation for the Layout canvas while outline
/// point editing is live.
pub(in crate::presentation) struct OutlineOverlay {
    pub outline_pitch: Vec2,
    pub outline_snap_origins: Rc<Vec<outline_snapping::Origin>>,
    pub outline_snap_settings: objects::LayoutSnapSettings,
    pub canvas_grid: objects::LayoutCanvasGridStyle,
    pub outline_overlay_key: Option<String>,
}

pub(in crate::presentation) fn use_outline_overlay(
    frame: &workspace_composition::WorkspaceFrame,
) -> OutlineOverlay {
    let adapter = use_context::<SelectionAdapter>();
    let interaction = use_context::<workspace_composition::WorkspaceInteraction>();
    let layout_state = use_context::<layout_workspace::LayoutWorkspaceState>();
    let model = frame.model.as_ref();
    let render_scope = &frame.scope;
    let tree_cell_anchor = &interaction.tree_cell_anchor;
    let layout_owner = &frame.owner;
    let accepted_canvas_source = frame.camera.accepted_canvas_source.clone();
    let layout_snap_settings = layout_state.snap_settings();
    let outline_inspector = &frame.layout_services.outline_inspector;
    let width = frame.camera.width;
    let zoom_surface_size = frame.surface_size;
    let outline_pitch = {
        let retained = tree_cell_anchor_for_owner(tree_cell_anchor, layout_owner);
        matrix_snap_parameters(model, render_scope, &adapter, retained.as_ref())
            .0
            .unwrap_or(Vec2 { x: 19.05, y: 19.05 })
    };
    let outline_snap_origins = use_memo(use_reactive((&accepted_canvas_source,), |(source,)| {
        Rc::new(layout_canvas::snap_origins(
            &source.document,
            &source.board_id,
            &source.scene,
        ))
    }))();
    let outline_snap_settings = layout_snap_settings.read().clone();
    let outline_grid_active = outline_inspector
        .as_ref()
        .is_some_and(|projection| (projection.editing_points)());
    let canvas_grid = objects::layout_canvas_grid_style(
        width,
        zoom_surface_size().0,
        outline_pitch.x,
        outline_snap_settings.snap_fraction,
        outline_grid_active,
    );
    let outline_overlay_key = outline_inspector
        .as_ref()
        .map(outline_lifecycle::OutlineInspectorProjection::canvas_edit_key);
    OutlineOverlay {
        outline_pitch,
        outline_snap_origins,
        outline_snap_settings,
        canvas_grid,
        outline_overlay_key,
    }
}

/// The canvas-space anchor for the selection transform menu: centroid x of
/// the selected parts, clamped into the view, above their lowest edge.
#[allow(clippy::too_many_arguments)]
pub(in crate::presentation) fn selection_geometry_anchor(
    visible: &[&Part],
    scene: &boardstudio_core::model::SceneDelta,
    selected_ids: &[String],
    svg_mounted: bool,
    zoom_surface_size: Signal<(f64, f64)>,
    width: f64,
    height: f64,
    view_x: f64,
) -> Option<(f64, f64, f64)> {
    let points: Vec<_> = visible
        .iter()
        .filter(|part| selected_ids.contains(&part.id))
        .map(|part| {
            scene
                .transforms
                .iter()
                .find(|t| t.id == part.id)
                .map_or(part.pose.at, |t| t.pose.at)
        })
        .collect();
    if points.is_empty() {
        return None;
    }
    let scale = if svg_mounted {
        let (surface_width, surface_height) = zoom_surface_size();
        (width / surface_width.max(1.0)).max(height / surface_height.max(1.0))
    } else {
        1.0
    };
    let x = (points.iter().map(|p| p.x).sum::<f64>() / points.len() as f64).clamp(
        view_x + 60.0 * scale,
        (view_x + width - 60.0 * scale).max(view_x + 60.0 * scale),
    );
    let y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min) - 12.0 - 6.0 * scale;
    Some((x, y, scale))
}
