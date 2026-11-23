//! Commit the captured placement and apply its workspace follow-up.
use super::*;

pub(super) fn commit_placement(
    host: &PartPlacementHost,
    state: PlacementState,
) -> impl FnMut(Vec2) + use<> {
    let runtime = host.runtime.clone();
    let PartPlacementHost {
        mut workspace,
        generation,
        mut layout_selection_kind,
        guide_preferences,
        ..
    } = *host;
    let PlacementState {
        mut active,
        mut committing,
        mut error,
        ..
    } = state;
    let mut adapter = host.adapter.clone();
    let canvas_interaction = use_context::<LayoutInteraction>();
    move |at: Vec2| {
        if canvas_interaction.current() != Some(CanvasMode::PlacingPart) {
            return;
        }
        let Some(mut placement) = active() else {
            return;
        };
        if *committing.read() {
            return;
        }
        update_pending_part(&mut placement.pending, at);
        placement
            .owner
            .record_committed_position(placement.pending.at);
        let model = runtime.model();
        if model.accepted.is_none() {
            return;
        }
        if !PlacementAdmission::capture(
            &runtime,
            generation(),
            workspace(),
            Workspace::Layout,
            guide_preferences(),
        )
        .admits(&placement.owner, &runtime, &model)
            || model.lifecycle != Lifecycle::Ready
            || model.durability
                != (Durability::Saved {
                    revision: placement.owner.revision,
                })
        {
            return;
        }
        let owner = placement.owner.clone();
        committing.set(true);
        let outcome = runtime.resolve_edit(
            "layout-component-placement",
            Some("placement".into()),
            placement_resolver(
                owner.board_id.clone(),
                placement.pending.definition.clone(),
                placement.pending.part.clone(),
                owner.layout_id.clone(),
                owner.intent,
            ),
        );
        active.set(None);
        error.set(None);
        let route_live = runtime.with_model(|model| {
            placement_route_is_current(
                &runtime,
                model,
                &owner,
                generation(),
                workspace(),
                guide_preferences(),
            )
        });
        let settlement = outcome.settlement(route_live);
        committing.set(false);
        let model = runtime.model();
        match settlement {
            Settlement::Landed { .. } => {
                // Select from the accepted document at the landing; nothing when the
                // placed part is no longer there.
                let placed = model.accepted.as_ref().is_some_and(|accepted| {
                    accepted
                        .document
                        .parts
                        .iter()
                        .any(|part| part.id == owner.part_id)
                });
                let next_context =
                    if owner.workflow == PlacementWorkflow::GeneralComponent && placed {
                        objects::context_for_part(&model, &owner.part_id).map(|context| {
                            ScopedTreeContext {
                                scope: owner.scope.clone(),
                                context,
                            }
                        })
                    } else {
                        None
                    };
                adapter.selected_context.set(next_context);
                adapter.anchor_scope.set(None);
                workspace.set(if owner.workflow == PlacementWorkflow::WiringController {
                    Workspace::Layout
                } else if owner.workflow.is_controller() {
                    Workspace::Pcb
                } else {
                    Workspace::Layout
                });
                if owner.workflow == PlacementWorkflow::GeneralComponent {
                    layout_selection_kind.set(objects::LayoutSelectionKind::Part);
                }
                runtime.submit(Event::SelectParts {
                    operation_id: runtime.operation(),
                    part_ids: if owner.workflow == PlacementWorkflow::GeneralComponent && placed {
                        vec![owner.part_id.clone()]
                    } else {
                        Vec::new()
                    },
                    range_part_ids: Vec::new(),
                    mode: SelectionMode::Replace,
                });
            }
            Settlement::Failed { message, .. } => {
                workspace.set(owner.source_workspace);
                error.set(Some(message));
            }
            Settlement::Retired => {}
        }
        canvas_interaction.end(CanvasMode::PlacingPart);
    }
}
