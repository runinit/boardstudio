//! Root-lifetime admission and exact outcome settlement for Layout Align commands.
use super::layout_align::{alignment_delta, reconcile_reference_choice, transformed_envelope};
use super::{
    AlignAction, AlignCommand, AlignFeedback, AlignReference, LayoutAlignMount, ScopedTreeContext,
    TreeContext,
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
#[cfg(test)]
use boardstudio_core::model::{EditCommand, EditPhase};
use boardstudio_core::model::{EditOperation, Part, ProjectDoc, Vec2};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
#[cfg(test)]
use boardstudio_web_runtime::pending_edits::PendingEdits;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

/// Domain data needed to place a terminal Align result in its original context.
#[derive(Clone)]
struct AlignFollowUp {
    workspace: &'static str,
    scope: Scope,
    scope_generation: u64,
    context: TreeContext,
    moving_ids: Vec<String>,
    reference_id: String,
    command: AlignCommand,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AlignActionKey {
    Align,
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
    let pending = use_hook(PendingEditSignals::<AlignActionKey>::new);
    let pending_align = use_signal(|| false);
    pending.bind_one_shot(AlignActionKey::Align, pending_align);
    let align_follow_up = use_signal(|| None::<AlignFollowUp>);
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
            let selected_reference = selected_reference;
            let pending = pending.clone();
            let mut align_follow_up = align_follow_up;
            let mut feedback = feedback;
            move |(_, workspace, scope_generation, identity, _reference_id)| {
                settle_pending(
                    &runtime,
                    selected_context,
                    &identity,
                    &pending,
                    &mut align_follow_up,
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
    let busy = pending_align();

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
        let pending = pending.clone();
        let mut align_follow_up = align_follow_up;
        let mut feedback = feedback;
        move |request: AlignAction| {
            if pending_align() {
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
            pending.begin_one_shot(
                &runtime,
                AlignActionKey::Align,
                "layout-align",
                Some("alignment".into()),
                align_resolver(
                    selected.context.clone(),
                    current.moving_ids.clone(),
                    reference_id.clone(),
                    request.command,
                ),
            );
            align_follow_up.set(Some(AlignFollowUp {
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
        busy,
        feedback: shown_feedback,
        on_reference,
        on_align,
    }
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

fn make_edit(
    document: &ProjectDoc,
    context: &TreeContext,
    moving_ids: &[String],
    delta: Vec2,
) -> Option<(EditOperation, Vec<String>)> {
    match context {
        TreeContext::Matrix { matrix_id } => {
            document
                .matrices
                .iter()
                .find(|matrix| matrix.id == *matrix_id)?;
            Some((
                EditOperation::SetMatrixPlacement {
                    matrix_id: matrix_id.clone(),
                    change: boardstudio_core::model::MatrixPlacementChange::MatrixTranslation {
                        delta,
                    },
                },
                vec![matrix_id.clone()],
            ))
        }
        TreeContext::Column { matrix_id, column } => {
            document
                .matrices
                .iter()
                .find(|matrix| matrix.id == *matrix_id)?;
            Some((
                EditOperation::SetMatrixPlacement {
                    matrix_id: matrix_id.clone(),
                    change: boardstudio_core::model::MatrixPlacementChange::ColumnTranslation {
                        column: *column,
                        delta,
                    },
                },
                vec![matrix_id.clone()],
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
            Some((EditOperation::MoveParts { positions }, moving_ids.to_vec()))
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
    identity: &AlignIdentity,
    pending: &PendingEditSignals<AlignActionKey>,
    align_follow_up: &mut Signal<Option<AlignFollowUp>>,
    feedback: &mut Signal<Option<AlignFeedbackState>>,
) {
    let Some(waiting) = align_follow_up.peek().clone() else {
        return;
    };
    let model = runtime.model();
    let live_scope = runtime.scope();
    let same_scope = live_scope.as_ref() == Some(&waiting.scope)
        && identity.scope_generation == waiting.scope_generation;
    let results = pending.settle(same_scope, |_| String::new());
    let Some(result) = results.into_iter().next() else {
        return;
    };
    align_follow_up.set(None);
    let (message, succeeded) = match result {
        PendingEditResult::Landed { .. } => (
            format!(
                "Aligned selection to {}. Reference unchanged.",
                reference_label(&model, &waiting.reference_id)
            ),
            true,
        ),
        PendingEditResult::Failed { message, .. } => (message, false),
        PendingEditResult::Retired { .. } => {
            feedback.set(None);
            return;
        }
    };
    let still_visible_target = identity.workspace == waiting.workspace
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
    use boardstudio_core::model::{
        Board, Follow, Layout, Matrix, MatrixAnchor, MatrixCell, MatrixPlacementChange,
        OutlineFeature, OutlineSettings, Pose2, Side,
    };
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn part(id: &str, x: f64) -> Part {
        Part {
            keycap: None,
            outline: None,
            anchor: None,
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
        let mut pending = PendingEdits::default();
        pending.begin(
            &runtime,
            (),
            "layout-align",
            Some("alignment".into()),
            align_resolver(context(), vec!["a".into()], "b".into(), AlignCommand::Left),
        );
        support::run_pending(&runtime).await;
        assert!(matches!(
            pending.settle(true).as_slice(),
            [PendingEditResult::Landed { .. }]
        ));
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
        support::submit_fixed_command(
            &runtime,
            EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: "delete-a".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["a".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(without_a),
                },
            },
        );
        support::drive_pending(&runtime);
        entered.await.expect("the delete reached Core");
        let mut pending = PendingEdits::default();
        pending.begin(
            &runtime,
            (),
            "layout-align",
            Some("alignment".into()),
            align_resolver(context(), vec!["a".into()], "b".into(), AlignCommand::Left),
        );
        assert!(
            pending.is_pending(&()),
            "a one-shot align stays pending until it settles"
        );
        support::drive_pending(&runtime);
        release.send(()).expect("release the held delete");
        for _ in 0..50 {
            support::run_pending(&runtime).await;
            if !pending.is_pending(&()) {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        match pending.settle(true).as_slice() {
            [PendingEditResult::Failed { message, .. }] => assert!(
                message.contains("no longer exists"),
                "the reason is explained: {message}"
            ),
            other => panic!("expected a retirement with a reason, got {other:?}"),
        }
        assert_eq!(pose_x(&runtime, "a"), None);
    }

    fn switch_definition() -> serde_json::Value {
        serde_json::json!({
            "id": "switch:base",
            "name": "MX switch",
            "kind": "switch",
            "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
            "pads": []
        })
    }

    fn key_part(id: &str, reference: &str, x: f64, y: f64) -> Part {
        Part {
            keycap: None,
            outline: None,
            anchor: None,
            id: id.into(),
            definition_id: "switch:base".into(),
            reference: reference.into(),
            pose: Pose2 {
                at: Vec2 { x, y },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn bent_column_document() -> ProjectDoc {
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
            part_ids: vec![
                "matrix/m/r0c0".into(),
                "matrix/m/r1c0".into(),
                "ref-part".into(),
            ],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document
            .definitions
            .push(serde_json::from_value(switch_definition()).unwrap());
        document
            .parts
            .push(key_part("matrix/m/r0c0", "SW1", 0.0, 0.0));
        document
            .parts
            .push(key_part("matrix/m/r1c0", "SW2", 0.0, 19.05));
        document.parts.push(key_part("ref-part", "REF", 40.0, 0.0));
        document.matrices.push(Matrix {
            id: "m".into(),
            name: None,
            rows: 2,
            columns: 1,
            pitch: Vec2 { x: 19.05, y: 19.05 },
            origin: Vec2::default(),
            definition_id: "switch:base".into(),
            part_ids: vec!["matrix/m/r0c0".into(), "matrix/m/r1c0".into()],
            board_id: Some("board-main".into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_layout: vec![],
            column_layout: vec![],
            anchor: None,
            cells: vec![
                MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row: 0,
                    column: 0,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                },
                MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row: 1,
                    column: 0,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                },
            ],
        });
        document.layouts.push(Layout {
            id: "layout".into(),
            name: "Main".into(),
            board_id: "board-main".into(),
            matrix_ids: vec!["m".into()],
            part_ids: vec!["ref-part".into()],
            mirror_link: None,
        });
        document
    }

    fn accepted_poses(runtime: &Rc<Runtime>, ids: &[&str]) -> Vec<Pose2> {
        let accepted = runtime.model().accepted.expect("an accepted document");
        ids.iter()
            .map(|id| {
                accepted
                    .document
                    .parts
                    .iter()
                    .find(|part| &part.id == id)
                    .unwrap_or_else(|| panic!("part {id} exists"))
                    .pose
            })
            .collect()
    }

    async fn commit_placement(runtime: &Rc<Runtime>, operation: EditOperation, target: &str) {
        let revision = runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .revision;
        support::submit_fixed_command(
            runtime,
            EditCommand {
                base_revision: revision,
                transaction_id: format!("ticket-22-{target}"),
                phase: EditPhase::Commit,
                target_ids: vec![target.to_owned()],
                operation,
            },
        );
        support::run_pending(runtime).await;
    }

    /// Ticket 22: the Align consumer submits Core's exact edits, never client
    /// column offsets. Column goes through `ColumnTranslation`, Matrix through its
    /// frame, Row stays unavailable, and keys keep the `MoveParts` pose route.
    #[wasm_bindgen_test::wasm_bindgen_test]
    fn align_submits_the_exact_core_edits() {
        let document = bent_column_document();
        let delta = Vec2 { x: -3.0, y: 1.5 };
        let moving = vec!["matrix/m/r0c0".into(), "matrix/m/r1c0".into()];
        match make_edit(
            &document,
            &TreeContext::Column {
                matrix_id: "m".into(),
                column: 0,
            },
            &moving,
            delta,
        ) {
            Some((
                EditOperation::SetMatrixPlacement {
                    matrix_id,
                    change: MatrixPlacementChange::ColumnTranslation { column, delta },
                },
                targets,
            )) => {
                assert_eq!(matrix_id, "m");
                assert_eq!(column, 0);
                assert_eq!(delta, Vec2 { x: -3.0, y: 1.5 });
                assert_eq!(targets, vec!["m".to_owned()]);
            }
            other => panic!("a bent column must Align rigidly, got {other:?}"),
        }
        match make_edit(
            &document,
            &TreeContext::Matrix {
                matrix_id: "m".into(),
            },
            &moving,
            delta,
        ) {
            Some((
                EditOperation::SetMatrixPlacement {
                    change: MatrixPlacementChange::MatrixTranslation { .. },
                    ..
                },
                _,
            )) => {}
            other => panic!("a matrix must Align through its frame, got {other:?}"),
        }
        assert!(
            make_edit(
                &document,
                &TreeContext::Row {
                    matrix_id: "m".into(),
                    row: 0,
                },
                &moving,
                delta,
            )
            .is_none(),
            "rows cross splayed columns and stay unaligned"
        );
        match make_edit(
            &document,
            &TreeContext::Key {
                matrix_id: "m".into(),
                row: 0,
                column: 0,
            },
            &moving[..1],
            delta,
        ) {
            Some((EditOperation::MoveParts { positions }, _)) => {
                assert_eq!(positions.len(), 1);
                assert_eq!(positions[0].at, Vec2 { x: -3.0, y: 1.5 });
            }
            other => panic!("keys keep the MoveParts pose route, got {other:?}"),
        }
    }

    /// Ticket 22 first slice: a Column Align on a bent column routes through Core's
    /// exact column edit (never client column offsets), so every target moves rigidly.
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn column_align_on_a_bent_column_moves_targets_rigidly() {
        let runtime = support::new_runtime();
        support::open_document(&runtime, bent_column_document()).await;
        commit_placement(
            &runtime,
            EditOperation::SetMatrixPlacement {
                matrix_id: "m".into(),
                change: MatrixPlacementChange::KeyBend {
                    row: 1,
                    column: 0,
                    value: 30.0,
                },
            },
            "m",
        )
        .await;
        let moving = vec!["matrix/m/r0c0".into(), "matrix/m/r1c0".into()];
        let before = accepted_poses(&runtime, &["matrix/m/r0c0", "matrix/m/r1c0"]);
        assert!(
            (before[0].rotation - before[1].rotation).abs() > 1.0,
            "the bend turns the second key: {before:?}"
        );
        let mut pending = PendingEdits::default();
        pending.begin(
            &runtime,
            (),
            "layout-align",
            Some("alignment".into()),
            align_resolver(
                TreeContext::Column {
                    matrix_id: "m".into(),
                    column: 0,
                },
                moving,
                "ref-part".into(),
                AlignCommand::Left,
            ),
        );
        support::run_pending(&runtime).await;
        match pending.settle(true).as_slice() {
            [PendingEditResult::Landed { .. }] => {}
            other => panic!("a rigid column align lands, got {other:?}"),
        }
        let after = accepted_poses(&runtime, &["matrix/m/r0c0", "matrix/m/r1c0"]);
        let delta = |index: usize| Vec2 {
            x: after[index].at.x - before[index].at.x,
            y: after[index].at.y - before[index].at.y,
        };
        let first = delta(0);
        let second = delta(1);
        assert!(
            first.x.hypot(first.y) > 1e-9,
            "the align moves the column: {before:?} -> {after:?}"
        );
        assert!(
            (first.x - second.x).abs() < 1e-9 && (first.y - second.y).abs() < 1e-9,
            "both bent-column targets share one rigid translation: {before:?} -> {after:?}"
        );
        assert_eq!(after[0].rotation, before[0].rotation);
        assert_eq!(after[1].rotation, before[1].rotation);
    }

    /// Ticket 22 first slice: an anchored/Follow refusal surfaces with the document
    /// and revision unchanged.
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn anchored_follow_refusal_leaves_the_document_unchanged() {
        let runtime = support::new_runtime();
        let mut document = bent_column_document();
        document.matrices.retain(|matrix| matrix.id == "m");
        document.matrices[0].rows = 1;
        document.matrices[0].columns = 2;
        document.matrices[0].part_ids = vec!["matrix/m/r0c0".into(), "matrix/m/r0c1".into()];
        document.matrices[0].cells = document.matrices[0]
            .cells
            .iter()
            .filter(|cell| cell.row == 0)
            .cloned()
            .collect();
        document.matrices[0].cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row: 0,
            column: 1,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
            assemblies_local: None,
        });
        document.parts.retain(|part| part.id != "matrix/m/r1c0");
        document
            .parts
            .push(key_part("matrix/m/r0c1", "SW2", 19.05, 0.0));
        document.boards[0].part_ids = vec![
            "matrix/m/r0c0".into(),
            "matrix/m/r0c1".into(),
            "ref-part".into(),
        ];
        support::open_document(&runtime, document).await;
        commit_placement(
            &runtime,
            EditOperation::AddAnchoredMatrix {
                id: "dep".into(),
                anchor: MatrixAnchor {
                    matrix_id: "m".into(),
                    row: 0,
                    column: 1,
                    follow: Follow {
                        x: false,
                        y: true,
                        rotation: true,
                    },
                    aggregate: None,
                },
            },
            "dep",
        )
        .await;
        let accepted = runtime.model().accepted.expect("an accepted document");
        let moving: Vec<String> = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .expect("the anchored matrix exists")
            .part_ids
            .clone();
        assert!(!moving.is_empty(), "the anchored matrix materializes parts");
        let revision = accepted.document.revision;
        let parts = accepted.document.parts.clone();
        let mut pending = PendingEdits::default();
        pending.begin(
            &runtime,
            (),
            "layout-align",
            Some("alignment".into()),
            align_resolver(
                TreeContext::Matrix {
                    matrix_id: "dep".into(),
                },
                moving,
                "ref-part".into(),
                AlignCommand::Left,
            ),
        );
        support::run_pending(&runtime).await;
        match pending.settle(true).as_slice() {
            [PendingEditResult::Failed { message, .. }] => assert!(
                message.contains("Follow"),
                "the refusal names the Follow restriction: {message}"
            ),
            other => panic!("an unfollowed align refuses, got {other:?}"),
        }
        let after = runtime.model().accepted.expect("an accepted document");
        assert_eq!(after.document.revision, revision);
        assert_eq!(after.document.parts, parts);
    }
}
