use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(super) struct CanvasSelectionStateInput<'a> {
    pub adapter: SelectionAdapter,
    pub render_scope: Scope,
    pub model: &'a ReadModel,
    pub layer_visibility: LayerVisibility,
    pub canvas_source: layout_canvas::SceneSource,
    pub spacing_ghost_state: Signal<Option<spacing_ghost::SpacingGhostState>>,
    pub snapshot: &'a AcceptedSnapshot,
    pub accepted_canvas_source: layout_canvas::SceneSource,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub active_workspace: Workspace,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub layout_owner: LayoutOwnerIdentity,
    pub layout_assembly_3d: Signal<bool>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
}

pub(super) struct CanvasSelectionState {
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

pub(super) fn use_canvas_selection_state(
    input: CanvasSelectionStateInput<'_>,
) -> CanvasSelectionState {
    let CanvasSelectionStateInput {
        adapter,
        render_scope,
        model,
        layer_visibility,
        canvas_source,
        spacing_ghost_state,
        snapshot,
        accepted_canvas_source,
        document,
        active_workspace,
        layout_snap_settings,
        layout_owner,
        layout_assembly_3d,
        layout_state,
    } = input;
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

pub(super) struct InspectorProjectionsInput<'a> {
    pub model: &'a ReadModel,
    pub active_workspace: Workspace,
    pub selected_tree_context: Option<objects::ScopedTreeContext>,
    pub layout_component_inspector_lifetime: Rc<inspector::LayoutComponentInspectorLifetime>,
    pub adapter: SelectionAdapter,
    pub accepted_token: Option<boardstudio_application::SnapshotToken>,
    pub current_scope: Option<Scope>,
    pub render_generation: u64,
    pub board_inspector: board_inspector::BoardInspectorMount,
    pub matrix_inspector: objects::MatrixInspectorMount,
    pub key_size: objects::KeySizeMount,
    pub matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub outline_inspector: Option<outline_lifecycle::OutlineInspectorProjection>,
    pub zoom_surface_size: Signal<(f64, f64)>,
    pub runtime: Rc<Runtime>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub part_placement: part_placement::PartPlacementMount,
    pub matrix_placement: objects::MatrixPlacementMount,
}

pub(super) struct InspectorProjections {
    pub component_inspector: Option<inspector::LayoutComponentInspectorProjection>,
    pub companion_placement: Option<inspector::CompanionPlacementProjection>,
    pub board_inspector_projection: Option<board_inspector::BoardInspectorProjection>,
    pub show_position_inspector: bool,
    pub context_summary: Option<context_summary::ContextSummary>,
    pub mount: EventHandler<MountedEvent>,
    pub canvas_aria_label: &'static str,
}

pub(super) fn use_inspector_projections(
    input: InspectorProjectionsInput<'_>,
) -> InspectorProjections {
    let InspectorProjectionsInput {
        model,
        active_workspace,
        selected_tree_context,
        layout_component_inspector_lifetime,
        adapter,
        accepted_token,
        current_scope,
        render_generation,
        board_inspector,
        matrix_inspector,
        key_size,
        matrix_transform_inspector,
        outline_inspector,
        zoom_surface_size,
        runtime,
        svg,
        part_placement,
        matrix_placement,
    } = input;
    let component_inspector_key = layout_component_inspector_owner_key(
        model,
        active_workspace,
        selected_tree_context.as_ref(),
    );
    let component_inspector_generation =
        layout_component_inspector_lifetime.update(component_inspector_key);
    let component_inspector_scope_generation = (adapter.generation)();
    let component_inspector_identity = (
        accepted_token,
        current_scope.clone(),
        model.active_board_id.clone(),
        model.active_instance_id.clone(),
        model.selected_part_ids.clone(),
        selected_tree_context.clone(),
        component_inspector_generation,
        component_inspector_scope_generation,
        active_workspace,
    );
    let component_inspector = use_memo(use_reactive((&component_inspector_identity,), {
        let runtime = runtime.clone();
        move |(identity,)| {
            let (
                token,
                _scope,
                active_board_id,
                active_instance_id,
                selected_part_ids,
                selected,
                context_generation,
                scope_generation,
                workspace,
            ) = identity;
            if workspace != Workspace::Layout {
                return None;
            }
            let model = runtime.model();
            if model.accepted.as_ref().map(|snapshot| snapshot.token) != token
                || model.active_board_id != active_board_id
                || model.active_instance_id != active_instance_id
                || model.selected_part_ids != selected_part_ids
            {
                return None;
            }
            layout_component_inspector_projection(
                &model,
                selected.as_ref(),
                context_generation,
                scope_generation,
            )
        }
    }))
    .read()
    .clone();
    let companion_placement = (active_workspace == Workspace::Layout)
        .then(|| {
            companion_placement_projection(
                model,
                selected_tree_context.as_ref(),
                render_generation,
                render_generation,
            )
        })
        .flatten();
    let board_inspector_projection = board_inspector.projection.clone().filter(|_| {
        component_inspector.is_none()
            && !matrix_inspector.is_active()
            && key_size.projection.is_none()
            && !matrix_transform_inspector.is_active()
            && outline_inspector.is_none()
    });
    let show_position_inspector = board_inspector_projection.is_none()
        && component_inspector.is_none()
        && selected_tree_context.as_ref().is_none_or(|selected| {
            let resolved = selection::resolve_context(model, &selected.context);
            resolved.is_some_and(|ids| {
                ids.len() == 1 && model.selected_part_ids.as_slice() == ids.as_slice()
            }) && matches!(&selected.context, objects::TreeContext::Component { .. })
        });
    let context_summary = selected_tree_context
        .as_ref()
        .and_then(|selected| context_summary::summarize(model, &selected.context));
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
    let canvas_aria_label = if matrix_placement.placement.is_some() || matrix_placement.busy {
        "Matrix placement canvas. Move the pointer or use arrow keys; click or press Enter to place, Escape to cancel."
    } else {
        "Keyboard layout; drag components, hold Shift for range selection, hold Space and drag to pan, or use position controls"
    };
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
    InspectorProjections {
        component_inspector,
        companion_placement,
        board_inspector_projection,
        show_position_inspector,
        context_summary,
        mount,
        canvas_aria_label,
    }
}

pub(super) struct CanvasNamed {
    pub selection: CanvasSelectionState,
    pub inspector: InspectorProjections,
}

pub(super) fn use_canvas_bundle(
    setup_a: &super::setup_bundles::SetupA,
    views: &super::setup_workspaces::Views,
    cam: &super::camera::CamNamed,
    snapshot: &AcceptedSnapshot,
    render_scope: &Scope,
) -> CanvasNamed {
    let adapter = &setup_a.adapter;
    let model = &setup_a.view.model;
    let layer_visibility = setup_a.editor.layers;
    let canvas_source = &cam.scene_bounds.canvas_source;
    let accepted_canvas_source = &cam.scene_bounds.accepted_canvas_source;
    let document = &views.document;
    let active_workspace = setup_a.active_workspace;
    let layout_snap_settings = setup_a.layout_snap_settings;
    let layout_owner = &setup_a.view.layout_owner;
    let layout_assembly_3d = setup_a.layout_assembly_3d;
    let layout_state = setup_a.layout_state;
    let layout_component_inspector_lifetime = &setup_a.layout_component_inspector_lifetime;
    let accepted_token = setup_a.view.layout_owner.token;
    let current_scope = &setup_a.view.current_scope;
    let render_generation = setup_a.render_generation;
    let board_inspector = &setup_a.board_inspector;
    let matrix_inspector = &setup_a.matrix_inspector;
    let key_size = &setup_a.key_size;
    let matrix_transform_inspector = &setup_a.matrix_transform_inspector;
    let outline_inspector = &setup_a.view.outline_inspector;
    let zoom_surface_size = cam.svg_surface.zoom_surface_size;
    let runtime = &setup_a.editor.runtime();
    let svg = &cam.svg_surface.svg;
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &setup_a.matrix_pair.matrix_placement;
    let spacing_ghost_state = setup_a.spacing_ghost_state;
    let CanvasSelectionState {
        selected_tree_context,
        canvas_anchor_badge,
        spacing_ghost_prop,
        layout_transform_target,
        part_rail_selection,
        part_rail_snap,
    } = use_canvas_selection_state(CanvasSelectionStateInput {
        adapter: adapter.clone(),
        render_scope: render_scope.clone(),
        model,
        layer_visibility,
        canvas_source: canvas_source.clone(),
        spacing_ghost_state,
        snapshot,
        accepted_canvas_source: accepted_canvas_source.clone(),
        document: document.clone(),
        active_workspace,
        layout_snap_settings,
        layout_owner: layout_owner.clone(),
        layout_assembly_3d,
        layout_state,
    });
    let InspectorProjections {
        component_inspector,
        companion_placement,
        board_inspector_projection,
        show_position_inspector,
        context_summary,
        mount,
        canvas_aria_label,
    } = use_inspector_projections(InspectorProjectionsInput {
        model,
        active_workspace,
        selected_tree_context: selected_tree_context.clone(),
        layout_component_inspector_lifetime: layout_component_inspector_lifetime.clone(),
        adapter: adapter.clone(),
        accepted_token,
        current_scope: current_scope.clone(),
        render_generation,
        board_inspector: board_inspector.clone(),
        matrix_inspector: matrix_inspector.clone(),
        key_size: key_size.clone(),
        matrix_transform_inspector: matrix_transform_inspector.clone(),
        outline_inspector: outline_inspector.clone(),
        zoom_surface_size,
        runtime: runtime.clone(),
        svg: svg.clone(),
        part_placement: part_placement.clone(),
        matrix_placement: matrix_placement.clone(),
    });
    CanvasNamed {
        selection: CanvasSelectionState {
            selected_tree_context,
            canvas_anchor_badge,
            spacing_ghost_prop,
            layout_transform_target,
            part_rail_selection,
            part_rail_snap,
        },
        inspector: InspectorProjections {
            component_inspector,
            companion_placement,
            board_inspector_projection,
            show_position_inspector,
            context_summary,
            mount,
            canvas_aria_label,
        },
    }
}
