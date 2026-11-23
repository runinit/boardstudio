//! Admission and synchronous execution for Layout Align commands.
use super::layout_align::{alignment_delta, reconcile_reference_choice, transformed_envelope};
use super::layout_align_logic::{make_edit, workspace_route_matches};
use super::{
    AlignAction, AlignCommand, AlignFeedback, AlignReference, LayoutAlignMount, ScopedTreeContext,
    TreeContext,
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_model::model::{Part, ProjectDoc, Vec2};
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

#[derive(Clone)]
struct AlignFeedbackState {
    scope: Scope,
    scope_generation: u64,
    context: TreeContext,
    moving_ids: Vec<String>,
    reference_id: String,
    feedback: AlignFeedback,
}

#[derive(Clone, PartialEq)]
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

#[derive(Clone, PartialEq)]
struct AlignProjectionIdentity {
    accepted_token: Option<boardstudio_application::SnapshotToken>,
    live_scope: Option<Scope>,
    active_board_id: String,
    active_instance_id: Option<String>,
    selected_ids: Vec<String>,
    lifecycle: Lifecycle,
    durability: Durability,
    has_preview: bool,
    has_gesture: bool,
    selected: Option<ScopedTreeContext>,
    workspace: &'static str,
    owner_workspace: &'static str,
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
    let feedback = use_signal(|| None::<AlignFeedbackState>);
    let selected = selected_context.read().clone();
    let current_workspace = workspace();
    let current_scope_generation = scope_generation();
    let model = runtime.model();
    let accepted_token = model.accepted.as_ref().map(|snapshot| snapshot.token);
    let active_board_id = model.active_board_id.clone();
    let active_instance_id = model.active_instance_id.clone();
    let selected_ids = model.selected_part_ids.clone();
    let lifecycle = model.lifecycle;
    let durability = model.durability.clone();
    let has_preview = model.display_preview.is_some();
    let has_gesture = model.gesture.is_some();
    let live_scope = runtime.scope();
    let projection_identity = AlignProjectionIdentity {
        accepted_token,
        live_scope,
        active_board_id,
        active_instance_id,
        selected_ids,
        lifecycle,
        durability,
        has_preview,
        has_gesture,
        selected: selected.clone(),
        workspace: current_workspace,
        owner_workspace,
    };
    let mut projection = use_memo(use_reactive((&projection_identity,), {
        let runtime = runtime.clone();
        move |(identity,)| {
            project(
                &runtime,
                identity.selected.as_ref(),
                identity.workspace,
                None,
                identity.owner_workspace,
            )
        }
    }))
    .read()
    .clone();

    use_effect(use_reactive(
        (
            &version(),
            &workspace(),
            &scope_generation(),
            &selected_reference.read().clone(),
        ),
        {
            let runtime = runtime.clone();
            let selected_reference = selected_reference;
            let mut feedback = feedback;
            move |(_, workspace, scope_generation, _reference_id)| {
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
        let mut feedback = feedback;
        move |request: AlignAction| {
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
                || !matches!(
                    model.durability,
                    Durability::Saved { revision } | Durability::Saving { revision }
                        if revision == snapshot.document.revision
                )
                || model.display_preview.is_some()
                || model.gesture.is_some()
                || model.active_board_id != selected.scope.board_id
                || model.active_instance_id != selected.scope.instance_id
            {
                feedback.set(None);
                return;
            }
            let result = runtime.commit(
                "layout-align",
                align_resolver(
                    selected.context.clone(),
                    current.moving_ids.clone(),
                    reference_id.clone(),
                    request.command,
                ),
            );
            let reference = reference_label(&runtime.model(), &reference_id);
            let succeeded = result.is_ok();
            let message = result.err().unwrap_or_else(|| {
                format!("Aligned selection to {reference}. Reference unchanged.")
            });
            feedback.set(Some(AlignFeedbackState {
                scope: selected.scope,
                scope_generation: current_generation,
                context: selected.context,
                moving_ids: current.moving_ids,
                reference_id,
                feedback: AlignFeedback {
                    reference,
                    command: request.command,
                    message,
                    succeeded,
                },
            }));
        }
    });

    let action = if projection.enabled {
        selected
            .as_ref()
            .zip(projection.selected_reference.as_ref())
            .map(|(selected, reference_id)| AlignAction {
                workspace: owner_workspace,
                scope: selected.scope.clone(),
                scope_generation: current_scope_generation,
                context: selected.context.clone(),
                moving_ids: projection.moving_ids.clone(),
                reference_id: reference_id.clone(),
                command: AlignCommand::Left,
            })
    } else {
        None
    };

    LayoutAlignMount {
        references: projection.references,
        selected_reference: projection.selected_reference,
        action,
        enabled: projection.enabled,
        disabled_reason: projection.disabled_reason,
        feedback: shown_feedback,
        on_reference,
        on_align,
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
        part.locked == Some(true) || snapshot.document.constraint_driving_part(&part.id) != Ok(None)
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
        && matches!(
            model.durability,
            Durability::Saved { revision } | Durability::Saving { revision }
                if revision == snapshot.document.revision
        )
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
    scenes: &[boardstudio_model::model::MatrixScene],
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
    let layout = document.layouts.iter().find(|layout| {
        layout.board_id == scope.board_id && layout.matrix_ids.iter().any(|id| *id == matrix_id)
    })?;
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
    layout
        .mirror_link
        .as_ref()
        .or(linked.mirror_link.as_ref())?
        .partner_matrix(matrix_id)
        .map(str::to_owned)
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
        if moving_parts.iter().any(|part| {
            part.locked == Some(true) || document.constraint_driving_part(&part.id) != Ok(None)
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
        Resolution::submit(target_ids, operation)
    })
}

fn reference_label(model: &boardstudio_application::ReadModel, id: &str) -> String {
    model
        .accepted
        .as_ref()
        .and_then(|snapshot| snapshot.document.parts.iter().find(|part| part.id == id))
        .map(|part| part.reference.clone())
        .unwrap_or_else(|| id.to_owned())
}
