//! Root-lifetime admission and exact outcome settlement for Layout Align commands.
use super::layout_align::{
    alignment_delta, local_matrix_delta, reconcile_reference_choice, transformed_envelope,
};
use super::{
    AlignAction, AlignCommand, AlignFeedback, AlignReference, LayoutAlignMount, ScopedTreeContext,
    TreeContext,
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, Part, ProjectDoc, Vec2,
};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

/// The in-flight Align: the edit ticket plus the identity its feedback is shown under.
/// One-shot, so the Align controls disable while the ticket is pending.
#[derive(Clone)]
struct PendingAlign {
    ticket: EditTicket,
    workspace: &'static str,
    scope: Scope,
    scope_generation: u64,
    context: TreeContext,
    moving_ids: Vec<String>,
    reference_id: String,
    command: AlignCommand,
}

#[derive(Clone)]
struct AlignFeedbackState {
    scope: Scope,
    scope_generation: u64,
    context: TreeContext,
    moving_ids: Vec<String>,
    reference_id: String,
    feedback: AlignFeedback,
}

#[derive(Clone)]
struct AlignProjection {
    references: Vec<AlignReference>,
    selected_reference: Option<String>,
    enabled: bool,
    disabled_reason: Option<String>,
    scope: Option<Scope>,
    context: Option<TreeContext>,
    moving_ids: Vec<String>,
    reference_authoritative: bool,
}

#[derive(Clone)]
struct AlignIdentity {
    scope: Option<Scope>,
    context: Option<TreeContext>,
    workspace: &'static str,
    scope_generation: u64,
}

impl PartialEq for AlignIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.scope == other.scope
            && self.context == other.context
            && self.workspace == other.workspace
            && self.scope_generation == other.scope_generation
    }
}

/// Must run at the Editor lifetime, including while the Layout toolbar is hidden.
pub fn use_canvas_align(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    owner_workspace: &'static str,
) -> LayoutAlignMount {
    let mut selected_reference = use_signal(|| None::<String>);
    let pending = use_signal(|| None::<PendingAlign>);
    let feedback = use_signal(|| None::<AlignFeedbackState>);
    let selected = selected_context.read().clone();
    let current_workspace = workspace();
    let current_scope_generation = scope_generation();
    let identity = AlignIdentity {
        scope: selected.as_ref().map(|selected| selected.scope.clone()),
        context: selected.as_ref().map(|selected| selected.context.clone()),
        workspace: current_workspace,
        scope_generation: current_scope_generation,
    };
    let mut projection = project(
        &runtime,
        selected.as_ref(),
        current_workspace,
        selected_reference.read().as_deref(),
        owner_workspace,
    );

    // Numeric version is deliberately read as a dependency so the exact request settles after
    // accepted operations even when the Align popover or Layout workspace is hidden.
    use_effect(use_reactive(
        (
            &version(),
            &workspace(),
            &scope_generation(),
            &identity,
            &selected_reference.read().clone(),
        ),
        {
            let runtime = runtime.clone();
            let mut selected_reference = selected_reference;
            let mut pending = pending;
            let mut feedback = feedback;
            move |(_, workspace, scope_generation, identity, _reference_id)| {
                reconcile_reference(
                    &runtime,
                    selected_context,
                    workspace,
                    scope_generation,
                    &mut selected_reference,
                    owner_workspace,
                );
                settle_pending(
                    &runtime,
                    selected_context,
                    workspace,
                    scope_generation,
                    &identity,
                    &mut pending,
                    &mut feedback,
                );
                clear_stale_feedback(
                    &runtime,
                    selected_context,
                    workspace,
                    scope_generation,
                    selected_reference,
                    &mut feedback,
                    owner_workspace,
                );
            }
        },
    ));

    let reference_id = selected_reference.peek().clone();
    let eligible_reference_ids: Vec<_> = projection
        .references
        .iter()
        .map(|reference| reference.id.clone())
        .collect();
    projection.selected_reference = reconcile_reference_choice(
        reference_id.as_deref(),
        &eligible_reference_ids,
        projection.reference_authoritative,
        |next| selected_reference.set(next),
    );
    let shown_feedback = feedback.read().clone().and_then(|state| {
        (selected
            .as_ref()
            .is_some_and(|selected| selected.context == state.context)
            && selected
                .as_ref()
                .is_some_and(|selected| selected.scope == state.scope)
            && current_scope_generation == state.scope_generation
            && projection.moving_ids == state.moving_ids
            && projection.selected_reference.as_deref() == Some(state.reference_id.as_str()))
        .then_some(state.feedback)
    });
    let busy = pending
        .read()
        .as_ref()
        .is_some_and(|waiting| waiting.ticket.is_pending());

    let on_reference = use_callback({
        let runtime = runtime.clone();
        let mut selected_reference = selected_reference;
        move |reference_id: String| {
            let selected = selected_context.peek().clone();
            let current = project(
                &runtime,
                selected.as_ref(),
                workspace(),
                None,
                owner_workspace,
            );
            if current
                .references
                .iter()
                .any(|reference| reference.id == reference_id)
            {
                selected_reference.set(Some(reference_id));
            }
        }
    });

    let on_align = use_callback({
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |request: AlignAction| {
            if pending
                .peek()
                .as_ref()
                .is_some_and(|waiting| waiting.ticket.is_pending())
            {
                return;
            }
            let current_workspace = workspace();
            let current_generation = scope_generation();
            let Some(selected) = selected_context.peek().clone() else {
                return;
            };
            let reference_id = request.reference_id.clone();
            let current = project(
                &runtime,
                Some(&selected),
                current_workspace,
                Some(&reference_id),
                owner_workspace,
            );
            if !current.enabled
                || !workspace_route_matches(current_workspace, owner_workspace)
                || request.workspace != owner_workspace
                || current.scope.as_ref() != Some(&selected.scope)
                || current.context.as_ref() != Some(&selected.context)
                || current.moving_ids.is_empty()
                || request.scope != selected.scope
                || request.scope_generation != current_generation
                || request.context != selected.context
                || request.moving_ids != current.moving_ids
                || current.selected_reference.as_deref() != Some(request.reference_id.as_str())
                || !current
                    .references
                    .iter()
                    .any(|reference| reference.id == request.reference_id)
                || selected_reference.peek().as_deref() != Some(request.reference_id.as_str())
            {
                feedback.set(None);
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if runtime.scope().as_ref() != Some(&selected.scope)
                || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: snapshot.document.revision,
                    })
                || model.display_preview.is_some()
                || model.gesture.is_some()
                || model.active_board_id != selected.scope.board_id
                || model.active_instance_id != selected.scope.instance_id
            {
                feedback.set(None);
                return;
            }
            let ticket = EditTicket::begin(
                &runtime,
                "layout-align",
                Some("alignment".into()),
                align_resolver(
                    selected.context.clone(),
                    current.moving_ids.clone(),
                    reference_id.clone(),
                    request.command,
                ),
            );
            pending.set(Some(PendingAlign {
                ticket,
                workspace: request.workspace,
                scope: selected.scope.clone(),
                scope_generation: current_generation,
                context: selected.context.clone(),
                moving_ids: current.moving_ids.clone(),
                reference_id: reference_id.clone(),
                command: request.command,
            }));
            feedback.set(None);
        }
    });

    projection = project(
        &runtime,
        selected.as_ref(),
        current_workspace,
        projection.selected_reference.as_deref(),
        owner_workspace,
    );
    let retained_reference = selected_reference.peek().clone();
    let eligible_reference_ids: Vec<_> = projection
        .references
        .iter()
        .map(|reference| reference.id.clone())
        .collect();
    projection.selected_reference = reconcile_reference_choice(
        retained_reference.as_deref(),
        &eligible_reference_ids,
        projection.reference_authoritative,
        |next| selected_reference.set(next),
    );
    let action = if projection.enabled {
        selected
            .as_ref()
            .zip(projection.selected_reference.as_ref())
            .map(
                |(selected, reference_id)| AlignAction {
                    workspace: owner_workspace,
                    scope: selected.scope.clone(),
                    scope_generation: current_scope_generation,
                    context: selected.context.clone(),
                    moving_ids: projection.moving_ids.clone(),
                    reference_id: reference_id.clone(),
                    command: AlignCommand::Left,
                },
            )
    } else {
        None
    };

    LayoutAlignMount {
        references: projection.references,
        selected_reference: projection.selected_reference,
        action,
        enabled: projection.enabled,
        disabled_reason: projection.disabled_reason,
        busy,
        feedback: shown_feedback,
        on_reference,
        on_align,
    }
}

fn reconcile_reference(
    runtime: &Runtime,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: &'static str,
    _scope_generation: u64,
    selected_reference: &mut Signal<Option<String>>,
    owner_workspace: &'static str,
) {
    if !workspace_route_matches(workspace, owner_workspace) {
        return;
    }
    let selected = selected_context.peek().clone();
    if selected.is_none() {
        return;
    }
    let current = project(runtime, selected.as_ref(), workspace, None, owner_workspace);
    if !current.reference_authoritative {
        return;
    }
    let eligible: Vec<_> = current
        .references
        .iter()
        .map(|reference| reference.id.clone())
        .collect();
    let current_reference = selected_reference.peek().clone();
    reconcile_reference_choice(current_reference.as_deref(), &eligible, true, |next| {
        selected_reference.set(next)
    });
}

fn workspace_route_matches(current: &str, owner: &str) -> bool {
    matches!(owner, "Layout" | "PCB") && current == owner
}

#[cfg(all(test, target_arch = "wasm32"))]
mod workspace_route_tests {
    use super::workspace_route_matches;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn align_route_is_exactly_scoped_to_its_workspace_owner() {
        assert!(workspace_route_matches("Layout", "Layout"));
        assert!(workspace_route_matches("PCB", "PCB"));
        assert!(!workspace_route_matches("PCB", "Layout"));
        assert!(!workspace_route_matches("Layout", "PCB"));
        assert!(!workspace_route_matches("Parts", "Parts"));
    }
}

fn clear_stale_feedback(
    runtime: &Runtime,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: &'static str,
    scope_generation: u64,
    selected_reference: Signal<Option<String>>,
    feedback: &mut Signal<Option<AlignFeedbackState>>,
    owner_workspace: &'static str,
) {
    let Some(current_feedback) = feedback.peek().clone() else {
        return;
    };
    let selected = selected_context.peek().clone();
    let current = project(
        runtime,
        selected.as_ref(),
        workspace,
        selected_reference.peek().as_deref(),
        owner_workspace,
    );
    let still_current = selected.as_ref().is_some_and(|selected| {
        selected.scope == current_feedback.scope
            && scope_generation == current_feedback.scope_generation
            && selected.context == current_feedback.context
            && current.moving_ids == current_feedback.moving_ids
            && current.selected_reference.as_deref() == Some(current_feedback.reference_id.as_str())
    });
    if !still_current {
        feedback.set(None);
    }
}

fn project(
    runtime: &Runtime,
    selected: Option<&ScopedTreeContext>,
    workspace: &'static str,
    selected_reference: Option<&str>,
    owner_workspace: &'static str,
) -> AlignProjection {
    let model = runtime.model();
    let empty = AlignProjection {
        references: Vec::new(),
        selected_reference: None,
        enabled: false,
        disabled_reason: None,
        scope: selected.map(|selected| selected.scope.clone()),
        context: selected.map(|selected| selected.context.clone()),
        moving_ids: Vec::new(),
        reference_authoritative: false,
    };
    if workspace != owner_workspace {
        return empty;
    }
    let (Some(selected), Some(scope), Some(snapshot)) =
        (selected, runtime.scope(), model.accepted.as_ref())
    else {
        return empty;
    };
    if scope != selected.scope
        || snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
    {
        return empty;
    }
    let Some(moving_ids) = super::resolve_selection(&model, &selected.context) else {
        return empty;
    };
    let selected_set: BTreeSet<_> = model.selected_part_ids.iter().collect();
    let resolved_set: BTreeSet<_> = moving_ids.iter().collect();
    if selected_set != resolved_set || moving_ids.is_empty() {
        return AlignProjection {
            moving_ids,
            ..empty
        };
    }
    let Some(board) = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)
    else {
        return AlignProjection {
            moving_ids,
            ..empty
        };
    };
    let visible: Vec<&Part> = snapshot
        .document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
        .collect();
    let board_part_ids: BTreeSet<_> = board.part_ids.iter().map(String::as_str).collect();
    let moving_matrix = context_matrix(&selected.context).or_else(|| {
        moving_ids.iter().find_map(|id| {
            snapshot
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.part_ids.contains(id))
                .map(|matrix| matrix.id.clone())
        })
    });
    let linked_partner_parts = moving_matrix
        .as_deref()
        .and_then(|matrix_id| linked_partner(&snapshot.document, &scope, matrix_id))
        .filter(|_| {
            linked_selection(
                &selected.context,
                &moving_ids,
                &snapshot.document,
                &snapshot.scene.matrix_scenes,
                &board.id,
            )
        })
        .and_then(|partner_id| {
            snapshot
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == partner_id)
        })
        .map(|matrix| matrix.part_ids.iter().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let moving_set: BTreeSet<_> = moving_ids.iter().cloned().collect();
    let references: Vec<_> = visible
        .iter()
        .filter(|part| !moving_set.contains(&part.id) && !linked_partner_parts.contains(&part.id))
        .map(|part| AlignReference {
            id: part.id.clone(),
            label: part.reference.clone(),
        })
        .collect();
    let selected_reference = selected_reference
        .filter(|id| references.iter().any(|reference| reference.id == *id))
        .map(str::to_owned)
        .or_else(|| references.first().map(|reference| reference.id.clone()));
    let moving_parts: Vec<_> = moving_ids
        .iter()
        .filter_map(|id| visible.iter().find(|part| part.id == *id).copied())
        .collect();
    if moving_parts.len() != moving_ids.len() {
        return AlignProjection {
            references,
            selected_reference,
            moving_ids,
            reference_authoritative: true,
            ..empty
        };
    }
    let locked_or_driven = moving_parts.iter().any(|part| {
        part.locked == Some(true)
            || snapshot.document.constraints.iter().any(|constraint| {
                board_part_ids.contains(constraint.source())
                    && board_part_ids.contains(constraint.target())
                    && constraint.target() == part.id
            })
    });
    let row_scope = matches!(selected.context, TreeContext::Row { .. });
    let supported_scope = matches!(
        selected.context,
        TreeContext::Matrix { .. }
            | TreeContext::Row { .. }
            | TreeContext::Column { .. }
            | TreeContext::Key { .. }
            | TreeContext::Component { .. }
    );
    let unsupported = unsupported_source(
        &snapshot.document,
        &moving_parts,
        selected_reference.as_deref(),
    );
    let active = model.lifecycle == Lifecycle::Ready
        && model.durability
            == (Durability::Saved {
                revision: snapshot.document.revision,
            })
        && model.display_preview.is_none()
        && model.gesture.is_none();
    let disabled_reason = if locked_or_driven {
        Some("Unlock or remove the driving constraint before aligning.".into())
    } else if row_scope {
        Some(
            "A row can cross differently splayed columns. Align individual keys or a column."
                .into(),
        )
    } else {
        unsupported.clone()
    };
    AlignProjection {
        references,
        selected_reference,
        enabled: active
            && supported_scope
            && !row_scope
            && !locked_or_driven
            && unsupported.is_none(),
        disabled_reason,
        scope: Some(scope),
        context: Some(selected.context.clone()),
        moving_ids,
        reference_authoritative: true,
    }
}

fn unsupported_source(
    document: &ProjectDoc,
    moving: &[&Part],
    reference_id: Option<&str>,
) -> Option<String> {
    for part in moving {
        let supported = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
            .is_some_and(|definition| transformed_envelope(part, definition).is_ok());
        if !supported {
            return Some(format!(
                "Unsupported alignment target: {} has no supported envelope.",
                part.reference
            ));
        }
    }
    if let Some(reference_id) = reference_id {
        let Some(part) = document.parts.iter().find(|part| part.id == reference_id) else {
            return Some("The chosen alignment reference is no longer available.".into());
        };
        if !document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
            .is_some_and(|definition| transformed_envelope(part, definition).is_ok())
        {
            return Some(format!(
                "Unsupported alignment reference: {} has no supported envelope.",
                part.reference
            ));
        }
    }
    None
}

fn context_matrix(context: &TreeContext) -> Option<String> {
    match context {
        TreeContext::Matrix { matrix_id }
        | TreeContext::Row { matrix_id, .. }
        | TreeContext::Column { matrix_id, .. }
        | TreeContext::Key { matrix_id, .. } => Some(matrix_id.clone()),
        TreeContext::Component { matrix_id, .. } => matrix_id.clone(),
        TreeContext::Board { .. }
        | TreeContext::LayoutGroup { .. }
        | TreeContext::Outline { .. }
        | TreeContext::OutlineVersion { .. }
        | TreeContext::Bridge { .. }
        | TreeContext::MountedModule { .. } => None,
    }
}

fn linked_selection(
    context: &TreeContext,
    moving_ids: &[String],
    document: &ProjectDoc,
    scenes: &[boardstudio_core::model::MatrixScene],
    board_id: &str,
) -> bool {
    match context {
        TreeContext::Component {
            assembly_id,
            matrix_id,
            part_id,
            ..
        } => {
            if assembly_id.is_some() {
                return false;
            }
            let Some(part_id) = part_id.as_deref() else {
                return false;
            };
            document.matrices.iter().any(|matrix| {
                matrix_id.as_deref().is_none_or(|id| id == matrix.id)
                    && matrix.part_ids.contains(&part_id.to_owned())
                    && document
                        .boards
                        .iter()
                        .find(|board| board.id == board_id)
                        .is_some_and(|board| {
                            board.part_ids.contains(&part_id.to_owned())
                                && matrix.part_ids.iter().any(|id| board.part_ids.contains(id))
                        })
                    && scenes
                        .iter()
                        .find(|scene| scene.matrix_id == matrix.id)
                        .is_some_and(|scene| {
                            scene.cells.iter().any(|cell| {
                                cell.enabled && cell.member_id.as_deref() == Some(part_id)
                            })
                        })
            })
        }
        _ => !moving_ids.is_empty(),
    }
}

fn linked_partner(document: &ProjectDoc, scope: &Scope, matrix_id: &str) -> Option<String> {
    let layout = document
        .layouts
        .iter()
        .find(|layout| layout.board_id == scope.board_id && layout.matrix_id == matrix_id)?;
    let linked = if let Some(link) = &layout.mirror_link {
        document.layouts.iter().find(|candidate| {
            candidate.board_id == scope.board_id && candidate.id == link.source_id
        })
    } else {
        document.layouts.iter().find(|candidate| {
            candidate.board_id == scope.board_id
                && candidate
                    .mirror_link
                    .as_ref()
                    .is_some_and(|link| link.source_id == layout.id)
        })
    }?;
    Some(linked.matrix_id.clone())
}

/// Resolve an align against the accepted document at execution time. Every part the
/// command moves, and the reference, must still exist and stay eligible; the delta and
/// the replacement operation come from the accepted poses, so an align queued behind
/// other edits applies on top of them.
fn align_resolver(
    context: TreeContext,
    moving_ids: Vec<String>,
    reference_id: String,
    command: AlignCommand,
) -> EditResolver {
    EditResolver::new("layout-align", move |accepted: &AcceptedSnapshot| {
        let document = &accepted.document;
        let Some(reference) = document.parts.iter().find(|part| part.id == reference_id) else {
            return Resolution::Retire("The alignment reference no longer exists.".into());
        };
        let moving_parts: Vec<&Part> = moving_ids
            .iter()
            .filter_map(|id| document.parts.iter().find(|part| part.id == *id))
            .collect();
        if moving_parts.len() != moving_ids.len() {
            return Resolution::Retire("A selected part no longer exists.".into());
        }
        let board_part_ids: BTreeSet<&str> = document
            .boards
            .iter()
            .flat_map(|board| board.part_ids.iter().map(String::as_str))
            .collect();
        if moving_parts.iter().any(|part| {
            part.locked == Some(true)
                || document.constraints.iter().any(|constraint| {
                    constraint.target() == part.id
                        && board_part_ids.contains(constraint.source())
                })
        }) {
            return Resolution::Retire(
                "A selected part is locked or driven by a relationship.".into(),
            );
        }
        if let Some(reason) = unsupported_source(document, &moving_parts, Some(&reference_id)) {
            return Resolution::Retire(reason);
        }
        let envelope = |part: &Part| {
            document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
                .ok_or_else(|| format!("{} has no supported alignment definition", part.reference))
                .and_then(|definition| transformed_envelope(part, definition))
        };
        let reference_envelope = match envelope(reference) {
            Ok(envelope) => envelope,
            Err(reason) => return Resolution::Retire(reason),
        };
        let moving_envelopes: Result<Vec<_>, _> =
            moving_parts.iter().map(|part| envelope(part)).collect();
        let moving_envelopes = match moving_envelopes {
            Ok(envelopes) => envelopes,
            Err(reason) => return Resolution::Retire(reason),
        };
        let delta = match alignment_delta(&moving_envelopes, &reference_envelope, command) {
            Ok(delta) => delta,
            Err(reason) => return Resolution::Retire(reason),
        };
        if delta == Vec2::default() {
            return Resolution::Unchanged;
        }
        let Some((operation, target_ids)) = make_edit(document, &context, &moving_ids, delta)
        else {
            return Resolution::Retire("The aligned selection no longer exists.".into());
        };
        Resolution::Submit(EditCommand {
            base_revision: 0,
            transaction_id: String::new(),
            phase: EditPhase::Commit,
            target_ids,
            operation,
        })
    })
}

fn make_edit(
    document: &ProjectDoc,
    context: &TreeContext,
    moving_ids: &[String],
    delta: Vec2,
) -> Option<(EditOperation, Vec<String>)> {
    match context {
        TreeContext::Matrix { matrix_id } => {
            let mut matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == *matrix_id)?
                .clone();
            matrix.origin.x += delta.x;
            matrix.origin.y += delta.y;
            let id = matrix.id.clone();
            Some((
                EditOperation::SetMatrix {
                    matrix,
                    definitions: None,
                },
                vec![id],
            ))
        }
        TreeContext::Column { matrix_id, column } => {
            let mut matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == *matrix_id)?
                .clone();
            let local = local_matrix_delta(&matrix, delta, *column as usize);
            let index = *column as usize;
            while matrix.column_offsets.len() <= index {
                matrix.column_offsets.push(Vec2::default());
            }
            matrix.column_offsets[index].x += local.x;
            matrix.column_offsets[index].y += local.y;
            let id = matrix.id.clone();
            Some((
                EditOperation::SetMatrix {
                    matrix,
                    definitions: None,
                },
                vec![id],
            ))
        }
        TreeContext::Row { .. } => None,
        TreeContext::Key { .. } | TreeContext::Component { .. } => {
            let positions = moving_ids
                .iter()
                .map(|id| {
                    let part = document.parts.iter().find(|part| part.id == *id)?;
                    Some(boardstudio_core::model::Position {
                        id: id.clone(),
                        at: Vec2 {
                            x: part.pose.at.x + delta.x,
                            y: part.pose.at.y + delta.y,
                        },
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Some((
                EditOperation::MoveParts { positions },
                moving_ids.to_vec(),
            ))
        }
        TreeContext::Board { .. }
        | TreeContext::LayoutGroup { .. }
        | TreeContext::Outline { .. }
        | TreeContext::OutlineVersion { .. }
        | TreeContext::Bridge { .. }
        | TreeContext::MountedModule { .. } => None,
    }
}

fn settle_pending(
    runtime: &Runtime,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: &'static str,
    scope_generation: u64,
    identity: &AlignIdentity,
    pending: &mut Signal<Option<PendingAlign>>,
    feedback: &mut Signal<Option<AlignFeedbackState>>,
) {
    let Some(waiting) = pending.peek().clone() else {
        return;
    };
    let model = runtime.model();
    let live_scope = runtime.scope();
    let same_scope =
        live_scope.as_ref() == Some(&waiting.scope) && scope_generation == waiting.scope_generation;
    let settlement = waiting.ticket.settlement(same_scope);
    let (message, succeeded) = match settlement {
        Settlement::Pending => return,
        Settlement::Landed { .. } => (
            format!(
                "Aligned selection to {}. Reference unchanged.",
                reference_label(&model, &waiting.reference_id)
            ),
            true,
        ),
        Settlement::Failed { message } => (message, false),
        Settlement::Retired => {
            feedback.set(None);
            pending.set(None);
            return;
        }
    };
    let still_visible_target = workspace == waiting.workspace
        && selected_context.peek().as_ref().is_some_and(|selected| {
            selected.scope == waiting.scope && selected.context == waiting.context
        });
    if still_visible_target {
        feedback.set(Some(AlignFeedbackState {
            scope: waiting.scope.clone(),
            scope_generation: waiting.scope_generation,
            context: waiting.context.clone(),
            moving_ids: waiting.moving_ids.clone(),
            reference_id: waiting.reference_id.clone(),
            feedback: AlignFeedback {
                reference: reference_label(&model, &waiting.reference_id),
                command: waiting.command,
                message,
                succeeded,
            },
        }));
    } else if identity.scope.as_ref() != Some(&waiting.scope) {
        feedback.set(None);
    }
    pending.set(None);
}

fn reference_label(model: &boardstudio_application::ReadModel, id: &str) -> String {
    model
        .accepted
        .as_ref()
        .and_then(|snapshot| snapshot.document.parts.iter().find(|part| part.id == id))
        .map(|part| part.reference.clone())
        .unwrap_or_else(|| id.to_owned())
}

#[cfg(all(test, target_arch = "wasm32"))]
mod resolver_tests {
    use super::*;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_core::model::{Board, OutlineFeature, OutlineSettings, Pose2, Side};
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn part(id: &str, x: f64) -> Part {
        Part {
            keycap: None,
            outline: None,
            id: id.into(),
            definition_id: "switch:base".into(),
            reference: id.to_uppercase(),
            pose: Pose2 {
                at: Vec2 { x, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn fixture() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Keyboard");
        document.outline.push(OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: OutlineSettings::default(),
            id: "envelope-main".into(),
            part_ids: vec![],
            margin: 4.0,
            operation: boardstudio_core::model::Operation::Add,
        });
        document.boards.push(Board {
            id: "board-main".into(),
            name: "Main".into(),
            outline_ids: vec!["envelope-main".into()],
            part_ids: vec!["a".into(), "b".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id": "switch:base",
                "name": "MX switch",
                "kind": "switch",
                "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}, {"x": -3.0, "y": 3.0}],
                "pads": []
            }))
            .unwrap(),
        );
        document.parts.push(part("a", 0.0));
        document.parts.push(part("b", 20.0));
        document
    }

    fn context() -> TreeContext {
        TreeContext::Key {
            matrix_id: "matrix-main".into(),
            row: 0,
            column: 0,
        }
    }

    fn pose_x(runtime: &Rc<Runtime>, id: &str) -> Option<f64> {
        runtime
            .model()
            .accepted
            .as_ref()?
            .document
            .parts
            .iter()
            .find(|part| part.id == id)
            .map(|part| part.pose.at.x)
    }

    #[wasm_bindgen_test]
    async fn align_moves_the_selection_against_the_accepted_reference() {
        let runtime = support::new_runtime();
        support::open_document(&runtime, fixture()).await;
        let ticket = EditTicket::begin(
            &runtime,
            "layout-align",
            Some("alignment".into()),
            align_resolver(context(), vec!["a".into()], "b".into(), AlignCommand::Left),
        );
        support::run_pending(&runtime).await;
        assert!(matches!(ticket.settlement(true), Settlement::Landed { .. }));
        assert_eq!(pose_x(&runtime, "a"), Some(20.0));
    }

    #[wasm_bindgen_test]
    async fn align_of_a_part_deleted_before_execution_retires_with_a_reason() {
        let runtime = support::new_runtime();
        support::open_document(&runtime, fixture()).await;
        let accepted = runtime.model().accepted.expect("the fixture opens");
        // Hold the delete's Core reply so the align queues behind it.
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let mut without_a = accepted.document.as_ref().clone();
        without_a.parts.retain(|part| part.id != "a");
        without_a.boards[0].part_ids.retain(|id| id != "a");
        runtime.submit(boardstudio_application::Event::Edit {
            operation_id: runtime.operation(),
            command: EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: "delete-a".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["a".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(without_a),
                },
            },
        });
        support::drive_pending(&runtime);
        entered.await.expect("the delete reached Core");
        let ticket = EditTicket::begin(
            &runtime,
            "layout-align",
            Some("alignment".into()),
            align_resolver(context(), vec!["a".into()], "b".into(), AlignCommand::Left),
        );
        assert!(ticket.is_pending(), "a one-shot align stays pending until it settles");
        support::drive_pending(&runtime);
        release.send(()).expect("release the held delete");
        for _ in 0..50 {
            support::run_pending(&runtime).await;
            if !ticket.is_pending() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        match ticket.settlement(true) {
            Settlement::Failed { message } => assert!(
                message.contains("no longer exists"),
                "the reason is explained: {message}"
            ),
            other => panic!("expected a retirement with a reason, got {other:?}"),
        }
        assert_eq!(pose_x(&runtime, "a"), None);
    }
}
