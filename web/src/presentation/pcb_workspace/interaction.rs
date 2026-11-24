use super::*;
use crate::presentation::workspace_composition::WorkspaceFrame;

pub(super) fn part_hit(
    editor: EditorContext,
    adapter: SelectionAdapter,
    frame: &WorkspaceFrame,
    interaction: &WorkspaceInteraction,
) -> EventHandler<PcbPartHit> {
    let runtime = editor.runtime();
    let workspace = editor.workspace;
    let instance_selection = editor.instance_selection;
    let scope = frame.scope.clone();
    let generation = frame.owner.generation;
    let token = frame.snapshot.token;
    let owner = frame.owner.clone();
    let tree_cell_anchor = interaction.tree_cell_anchor.clone();
    let layout_selection_kind = frame.status.layout.selection_kind();
    EventHandler::new(move |request: pcb_scene::PcbPartHit| {
        if workspace() != Workspace::Pcb
            || request.scope != scope
            || request.token != token
            || request.generation != generation
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
            .filter(|snapshot| snapshot.token == request.token)
        else {
            return;
        };
        let Some(board) = snapshot
            .document
            .boards
            .iter()
            .find(|board| board.id == scope.board_id)
        else {
            return;
        };
        if !board.part_ids.contains(&request.part_id)
            || !snapshot
                .document
                .parts
                .iter()
                .any(|part| part.id == request.part_id)
        {
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
        let context = objects::context_for_selection_kind(
            &model,
            &hit_context,
            layout_selection_kind(),
            retained.as_ref(),
        )
        .map(|projection| projection.context)
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
        selection::submit_canvas_selection(
            &runtime, &adapter, &scope, generation, context, mode, range_ids,
        );
    })
}

pub(super) fn part_pointer_down(
    editor: EditorContext,
    adapter: SelectionAdapter,
    frame: &WorkspaceFrame,
    interaction: &WorkspaceInteraction,
) -> EventHandler<PcbPartPointerDown> {
    let runtime = editor.runtime();
    let workspace = editor.workspace;
    let instance_selection = editor.instance_selection;
    let scope = frame.scope.clone();
    let generation = frame.owner.generation;
    let svg = frame.svg.clone();
    let owner = frame.owner.clone();
    let accepted_token = frame.snapshot.token;
    let accepted_revision = frame.snapshot.document.revision;
    let tree_cell_anchor = interaction.tree_cell_anchor.clone();
    let canvas_interaction = use_context::<LayoutInteraction>();
    let (view_x, view_y, width, height) = (
        frame.camera.view_x,
        frame.camera.view_y,
        frame.camera.width,
        frame.camera.height,
    );
    let layout_selection_kind = frame.status.layout.selection_kind();
    EventHandler::new(move |request: pcb_scene::PcbPartPointerDown| {
        if request.scope != scope
            || request.token != accepted_token
            || request.generation != generation
            || !pcb_owner_is_current(&runtime, workspace, &adapter, &owner)
            || !instance_selection.is_current(&runtime.model())
            || canvas_interaction.current().is_some()
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
        let client = (f64::from(request.client_x), f64::from(request.client_y));
        if canvas_interaction.space_down() {
            super::workbench::editor_session::canvas_adapter::capture_canvas_drag(
                canvas_interaction,
                super::workbench::editor_session::canvas_adapter::CanvasDragStart {
                    mode: CanvasMode::Panning,
                    id: request.pointer_id as i32,
                    client,
                    control: false,
                    seed: Some(
                        super::workbench::editor_session::canvas_adapter::CanvasDragSeed::Pan {
                            scope: scope.clone(),
                            generation,
                            start_camera: runtime.with_model(|model| model.camera.center),
                        },
                    ),
                },
                &runtime,
                &svg,
            );
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
            || !matches!(current.durability, Durability::Saved { revision } | Durability::Saving { revision } if revision == current_snapshot.document.revision)
            || current.display_preview.is_some()
            || current.gesture.is_some()
        {
            return;
        }
        if !super::workbench::editor_session::canvas_adapter::capture_canvas_drag(
            canvas_interaction,
            super::workbench::editor_session::canvas_adapter::CanvasDragStart {
                mode: CanvasMode::DraggingParts,
                id: request.pointer_id as i32,
                client: (f64::from(request.client_x), f64::from(request.client_y)),
                control: false,
                seed: Some(
                    super::workbench::editor_session::canvas_adapter::CanvasDragSeed::Parts {
                        scope: scope.clone(),
                        generation,
                        origin_mm: origin,
                        positions,
                    },
                ),
            },
            &runtime,
            &svg,
        ) {
            return;
        }
    })
}

pub(super) fn empty_hit(
    editor: EditorContext,
    adapter: SelectionAdapter,
    frame: &WorkspaceFrame,
) -> EventHandler<PointerEvent> {
    let runtime = editor.runtime();
    let workspace = editor.workspace;
    let instance_selection = editor.instance_selection;
    let scope = frame.scope.clone();
    let generation = frame.owner.generation;
    let token = frame.snapshot.token;
    let canvas_interaction = use_context::<LayoutInteraction>();
    EventHandler::new(move |event: PointerEvent| {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        if pointer.button() != 0
            || canvas_interaction.current().is_some()
            || canvas_interaction.space_down()
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
        runtime.submit(boardstudio_application::Event::SelectParts {
            operation_id: runtime.operation(),
            part_ids: Vec::new(),
            range_part_ids: Vec::new(),
            mode: SelectionMode::Replace,
        });
    })
}
