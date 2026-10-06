//! Fresh accepted-source admission and exact operation acknowledgement for matrix fields.
use super::matrix_inspector::{
    MatrixDeleteRequest, MatrixDuplicateRequest, MatrixEditFeedback, MatrixEditField,
    MatrixEditRequest, MatrixEditState, MatrixEditValue, MatrixInspectorOwner,
    MatrixInspectorProjection, MatrixLayoutRelation, MatrixNameTarget, MatrixPreset,
    MatrixPresetRequest, MatrixUnlinkRequest, SwitchOrientation,
};
use super::{ScopedTreeContext, TreeContext};
use crate::{operation_outcomes::OutcomeSlot, runtime::Runtime};
use boardstudio_application::{
    Durability, Event, Lifecycle, OperationId, Scope, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::model::{
    DiodeDirection, EditCommand, EditOperation, EditPhase, Matrix, MatrixAssembly, MatrixCell,
    PartDefinition, ProjectDoc, Side, Vec2,
};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
struct PendingMatrixEdit {
    request: MatrixEditRequest,
    operation_id: OperationId,
    outcome: OutcomeSlot,
    base_token: SnapshotToken,
    base_revision: u64,
}

#[derive(Clone)]
struct PendingMatrixPreset {
    request: MatrixPresetRequest,
    operation_id: OperationId,
    outcome: OutcomeSlot,
    base_token: SnapshotToken,
    base_revision: u64,
    expected_variant: String,
}

#[derive(Clone)]
struct PendingMatrixDelete {
    request: MatrixDeleteRequest,
    outcome: OutcomeSlot,
    base_token: SnapshotToken,
    base_revision: u64,
}

#[derive(Clone)]
struct VariantReportOwner {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContextIdentity {
    scope: Scope,
    context: TreeContext,
    workspace: &'static str,
    scope_generation: u64,
}

#[derive(Default)]
struct ContextGeneration {
    initialized: bool,
    identity: Option<ContextIdentity>,
    value: u64,
}

/// Projection and exact callback state to mount beside the selected Layout context.
#[derive(Clone, PartialEq)]
pub(in crate::presentation) struct MatrixInspectorMount {
    pub projection: Option<MatrixInspectorProjection>,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub busy: bool,
    pub feedback: Vec<MatrixEditFeedback>,
    pub on_edit: EventHandler<MatrixEditRequest>,
    pub on_add_row: EventHandler<()>,
    pub on_add_column: EventHandler<()>,
    pub on_apply_preset: EventHandler<MatrixPresetRequest>,
    pub on_delete: EventHandler<MatrixDeleteRequest>,
    pub on_unlink: EventHandler<MatrixUnlinkRequest>,
    pub on_duplicate: EventHandler<MatrixDuplicateRequest>,
}

/// Must be called unconditionally at the shared presentation lifetime. It owns no document
/// state: every projection and callback resolves the current accepted snapshot again.
pub(in crate::presentation) fn use_matrix_inspector(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
) -> MatrixInspectorMount {
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let context_generation = use_hook(|| Rc::new(RefCell::new(ContextGeneration::default())));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    let selected = selected_context.read().clone();
    let current_workspace = workspace();
    let current_scope_generation = scope_generation();
    let observed_identity = selected.as_ref().map(|selected| ContextIdentity {
        scope: selected.scope.clone(),
        context: selected.context.clone(),
        workspace: current_workspace,
        scope_generation: current_scope_generation,
    });
    let generation = {
        let mut tracker = context_generation.borrow_mut();
        if !tracker.initialized || tracker.identity != observed_identity {
            tracker.value = tracker
                .value
                .checked_add(1)
                .expect("matrix inspector context generation exhausted");
            tracker.identity = observed_identity.clone();
            tracker.initialized = true;
        }
        tracker.value
    };

    let request_sequence = use_signal(|| 0u64);
    let last_request_id = use_signal(|| 0u64);
    let pending = use_signal(|| None::<PendingMatrixEdit>);
    let pending_preset = use_signal(|| None::<PendingMatrixPreset>);
    let pending_delete = use_signal(|| None::<PendingMatrixDelete>);
    let preparing_preset = use_signal(|| false);
    let duplicating = use_signal(|| false);
    let feedback = use_signal(Vec::<MatrixEditFeedback>::new);
    let switch_catalog = use_signal(Vec::<PartDefinition>::new);
    let mut switch_catalog_loaded = use_signal(|| false);
    let catalog_alive = alive.clone();
    use_effect(move || {
        if switch_catalog_loaded() {
            return;
        }
        switch_catalog_loaded.set(true);
        let mut switch_catalog = switch_catalog;
        let alive = catalog_alive.clone();
        spawn_local(async move {
            if let Ok(definitions) = crate::presentation::parts::load_matrix_templates(false).await
                && alive.get()
            {
                switch_catalog.set(definitions);
            }
        });
    });
    let (projection, editable) = project_current(
        &runtime,
        selected.as_ref(),
        editor_instance_id,
        generation,
        current_scope_generation,
        current_workspace,
    );
    let projection = projection.map(|mut projection| {
        if let Some(document) = runtime.model().accepted.map(|snapshot| snapshot.document) {
            projection.switch_choices =
                switch_choices(&document, &switch_catalog(), &projection.definition_id);
        }
        projection
    });
    let identity = observed_identity.clone();

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation(), &identity),
        {
            let runtime = runtime.clone();
            let mut pending = pending;
            let mut pending_preset = pending_preset;
            let mut pending_delete = pending_delete;
            let mut feedback = feedback;
            move |(_, _, scope_generation, _)| {
                settle_pending(&runtime, scope_generation, &mut pending, &mut feedback);
                settle_pending_preset(
                    &runtime,
                    scope_generation,
                    &mut pending_preset,
                    &mut feedback,
                );
                settle_pending_delete(
                    &runtime,
                    scope_generation,
                    &mut pending_delete,
                    selected_context,
                );
            }
        },
    ));

    let on_edit = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        let mut last_request_id = last_request_id;
        let mut pending = pending;
        let mut feedback = feedback;
        move |request: MatrixEditRequest| {
            let current_generation = context_generation.borrow().value;
            let current_workspace = workspace();
            let current_scope_generation = scope_generation();
            if request.owner.editor_instance_id != editor_instance_id
                || request.owner.context_generation != current_generation
                || request.owner.scope_generation != current_scope_generation
                || current_workspace != "Layout"
                || request.request_id <= last_request_id()
            {
                return;
            }
            last_request_id.set(request.request_id);
            if pending.read().is_some()
                || pending_preset.read().is_some()
                || pending_delete.read().is_some()
                || preparing_preset()
                || duplicating()
            {
                // Keep the active request's exact feedback, but settle this distinct rejected
                // attempt so its field draft cannot remain visually pending forever.
                publish_feedback(
                    &mut feedback,
                    &request,
                    MatrixEditState::Failed,
                    Some("Wait for the current matrix change to finish, then retry.".into()),
                );
                return;
            }
            let Some(selected) = selected_context.read().clone() else {
                return;
            };
            let live_scope = runtime.scope();
            let model = runtime.model();
            let Some((current, editable)) = project_current_for(
                &runtime,
                &model,
                live_scope.as_ref(),
                Some(&selected),
                MatrixProjectionContext {
                    editor_instance_id,
                    context_generation: current_generation,
                    scope_generation: current_scope_generation,
                    workspace: current_workspace,
                },
            ) else {
                return;
            };
            if current.owner != request.owner {
                return;
            }
            if !editable {
                publish_feedback(
                    &mut feedback,
                    &request,
                    MatrixEditState::Failed,
                    Some("The matrix is not ready to edit. Wait for the current operation to finish, then retry.".into()),
                );
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let Some(current_baseline) = field_value(
                &snapshot.document,
                &request.owner.matrix_id,
                &request.owner.name_target,
                request.field,
            ) else {
                return;
            };
            if current_baseline != request.baseline {
                publish_feedback(
                    &mut feedback,
                    &request,
                    MatrixEditState::Failed,
                    Some(
                        "The accepted value changed. Press Escape to reload it before editing."
                            .into(),
                    ),
                );
                return;
            }
            if request.value == request.baseline {
                publish_feedback(&mut feedback, &request, MatrixEditState::Saved, None);
                return;
            }
            let Some(matrix) = snapshot
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == request.owner.matrix_id)
            else {
                return;
            };
            let mut operation = match build_operation(
                &snapshot.document,
                matrix,
                &request.owner.name_target,
                request.field,
                request.value.clone(),
            ) {
                Ok(operation) => operation,
                Err(message) => {
                    publish_feedback(
                        &mut feedback,
                        &request,
                        MatrixEditState::Failed,
                        Some(message),
                    );
                    return;
                }
            };
            if let (
                MatrixEditField::SwitchDefinition,
                MatrixEditValue::SwitchDefinition(definition_id),
                EditOperation::SetMatrix { definitions, .. },
            ) = (&request.field, &request.value, &mut operation)
                && !snapshot
                    .document
                    .definitions
                    .iter()
                    .any(|definition| definition.id == *definition_id)
            {
                let Some(definition) = switch_catalog()
                    .into_iter()
                    .find(|definition| definition.id == *definition_id)
                else {
                    publish_feedback(
                        &mut feedback,
                        &request,
                        MatrixEditState::Failed,
                        Some("The selected switch footprint is not available in the loaded catalogue.".into()),
                    );
                    return;
                };
                *definitions = Some(vec![definition]);
            }
            let target_id = match &request.owner.name_target {
                MatrixNameTarget::Matrix => request.owner.matrix_id.clone(),
                MatrixNameTarget::Layout { id } if request.field == MatrixEditField::Name => {
                    id.clone()
                }
                MatrixNameTarget::Layout { .. } => request.owner.matrix_id.clone(),
            };
            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            let transaction_id = format!(
                "matrix-inspector-{}-{}-{}",
                editor_instance_id, request.request_id, operation_id.0
            );
            pending.set(Some(PendingMatrixEdit {
                request: request.clone(),
                operation_id,
                outcome,
                base_token: snapshot.token,
                base_revision: snapshot.document.revision,
            }));
            publish_feedback(&mut feedback, &request, MatrixEditState::Pending, None);
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id,
                    phase: EditPhase::Commit,
                    target_ids: vec![target_id],
                    operation,
                },
            });
        }
    });

    let preset_alive = alive.clone();
    let on_apply_preset = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        let mut last_request_id = last_request_id;
        let mut preparing_preset = preparing_preset;
        let mut feedback = feedback;
        let alive = preset_alive;
        move |request: MatrixPresetRequest| {
            if request.owner.editor_instance_id != editor_instance_id
                || request.owner.context_generation != context_generation.borrow().value
                || request.owner.scope_generation != scope_generation()
                || workspace() != "Layout"
                || request.request_id <= last_request_id()
            {
                return;
            }
            last_request_id.set(request.request_id);
            if pending.read().is_some()
                || pending_preset.read().is_some()
                || pending_delete.read().is_some()
                || preparing_preset()
                || duplicating()
            {
                publish_action_feedback(
                    &mut feedback,
                    &request.owner,
                    request.request_id,
                    MatrixEditField::ApplyPreset,
                    MatrixEditState::Failed,
                    Some("Wait for the current matrix change to finish, then retry.".into()),
                );
                return;
            }
            let selected = selected_context.read().clone();
            let model = runtime.model();
            let live_scope = runtime.scope();
            let Some((projection, editable)) = project_current_for(
                &runtime,
                &model,
                live_scope.as_ref(),
                selected.as_ref(),
                MatrixProjectionContext {
                    editor_instance_id,
                    context_generation: context_generation.borrow().value,
                    scope_generation: scope_generation(),
                    workspace: workspace(),
                },
            ) else {
                return;
            };
            if projection.owner != request.owner {
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if !editable
                || snapshot.token != request.snapshot_token
                || snapshot.document.revision != request.revision
                || projection.baseline_variant != request.baseline_variant
            {
                publish_action_feedback(
                    &mut feedback,
                    &request.owner,
                    request.request_id,
                    MatrixEditField::ApplyPreset,
                    MatrixEditState::Failed,
                    Some("The accepted matrix changed. Reopen the inspector before applying the preset.".into()),
                );
                return;
            }
            let Some(matrix) = snapshot
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == request.owner.matrix_id)
                .cloned()
            else {
                return;
            };
            let document = (*snapshot.document).clone();
            let base_token = snapshot.token;
            let base_revision = snapshot.document.revision;
            let reversible = is_reversible(&document);
            preparing_preset.set(true);
            publish_action_feedback(
                &mut feedback,
                &request.owner,
                request.request_id,
                MatrixEditField::ApplyPreset,
                MatrixEditState::Pending,
                None,
            );
            let runtime = runtime.clone();
            let mut preparing = preparing_preset;
            let mut pending = pending_preset;
            let mut feedback = feedback;
            let selected_context = selected_context;
            let context_generation = context_generation.clone();
            let alive = alive.clone();
            let workspace = workspace;
            let scope_generation = scope_generation;
            spawn_local(async move {
                let result = prepare_preset_matrix(
                    &document,
                    &matrix,
                    &request.owner.scope.board_id,
                    request.preset,
                    request.orientation,
                    reversible,
                )
                .await;
                if !alive.get() {
                    return;
                }
                preparing.set(false);
                let (replacement, definitions) = match result {
                    Ok(prepared) => prepared,
                    Err(message) => {
                        publish_action_feedback(
                            &mut feedback,
                            &request.owner,
                            request.request_id,
                            MatrixEditField::ApplyPreset,
                            MatrixEditState::Failed,
                            Some(message),
                        );
                        return;
                    }
                };
                let selected = selected_context.read().clone();
                let model = runtime.model();
                let scope = runtime.scope();
                let Some((current, editable)) = project_current_for(
                    &runtime,
                    &model,
                    scope.as_ref(),
                    selected.as_ref(),
                    MatrixProjectionContext {
                        editor_instance_id,
                        context_generation: context_generation.borrow().value,
                        scope_generation: scope_generation(),
                        workspace: workspace(),
                    },
                ) else {
                    publish_action_feedback(
                        &mut feedback,
                        &request.owner,
                        request.request_id,
                        MatrixEditField::ApplyPreset,
                        MatrixEditState::Failed,
                        Some("The matrix selection changed before the preset was ready.".into()),
                    );
                    return;
                };
                let Some(snapshot) = model.accepted.as_ref() else {
                    return;
                };
                if current.owner != request.owner
                    || !editable
                    || snapshot.token != base_token
                    || snapshot.document.revision != base_revision
                    || current.baseline_variant != request.baseline_variant
                {
                    publish_action_feedback(
                        &mut feedback,
                        &request.owner,
                        request.request_id,
                        MatrixEditField::ApplyPreset,
                        MatrixEditState::Failed,
                        Some("The accepted matrix changed before the preset was ready. Review it and try again.".into()),
                    );
                    return;
                }
                if !alive.get() {
                    return;
                }
                let operation_id = runtime.operation();
                let outcome = runtime.observe_operation(operation_id);
                pending.set(Some(PendingMatrixPreset {
                    request: request.clone(),
                    operation_id,
                    outcome: outcome.clone(),
                    base_token,
                    base_revision,
                    expected_variant: preset_variant(
                        request.preset,
                        request.orientation,
                        reversible,
                    ),
                }));
                runtime.submit(Event::Edit {
                    operation_id,
                    command: EditCommand {
                        base_revision,
                        transaction_id: format!(
                            "matrix-inspector-preset-{}-{}-{}",
                            editor_instance_id, request.request_id, operation_id.0
                        ),
                        phase: EditPhase::Commit,
                        target_ids: vec![request.owner.matrix_id.clone()],
                        operation: EditOperation::SetMatrix {
                            matrix: replacement,
                            definitions: Some(definitions),
                        },
                    },
                });
                while alive.get() && outcome.borrow().is_none() {
                    gloo_timers::future::TimeoutFuture::new(16).await;
                }
            });
        }
    });

    let on_delete = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        let mut pending_delete = pending_delete;
        move |request: MatrixDeleteRequest| {
            if request.owner.editor_instance_id != editor_instance_id
                || request.owner.context_generation != context_generation.borrow().value
                || request.owner.scope_generation != scope_generation()
                || workspace() != "Layout"
                || pending.read().is_some()
                || pending_preset.read().is_some()
                || pending_delete.read().is_some()
                || preparing_preset()
                || duplicating()
            {
                return;
            }
            let selected = selected_context.read().clone();
            let model = runtime.model();
            let scope = runtime.scope();
            let Some((projection, editable)) = project_current_for(
                &runtime,
                &model,
                scope.as_ref(),
                selected.as_ref(),
                MatrixProjectionContext {
                    editor_instance_id,
                    context_generation: context_generation.borrow().value,
                    scope_generation: scope_generation(),
                    workspace: workspace(),
                },
            ) else {
                return;
            };
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if projection.owner != request.owner
                || !editable
                || snapshot.token != request.snapshot_token
                || snapshot.document.revision != request.revision
            {
                runtime.report(
                    "The selected matrix changed. Reopen the inspector before deleting it.",
                );
                return;
            }
            let Some(matrix) = snapshot
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == request.owner.matrix_id)
            else {
                return;
            };
            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            pending_delete.set(Some(PendingMatrixDelete {
                request: request.clone(),
                outcome,
                base_token: snapshot.token,
                base_revision: snapshot.document.revision,
            }));
            let mut target_ids = vec![matrix.id.clone()];
            target_ids.extend(matrix.part_ids.iter().cloned());
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id: format!("matrix-inspector-delete-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids,
                    operation: EditOperation::RemoveMatrix {
                        id: matrix.id.clone(),
                    },
                },
            });
        }
    });

    let duplicate_alive = alive.clone();
    let on_duplicate = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        let mut duplicating = duplicating;
        let alive = duplicate_alive;
        move |request: MatrixDuplicateRequest| {
            if request.owner.editor_instance_id != editor_instance_id
                || request.owner.context_generation != context_generation.borrow().value
                || request.owner.scope_generation != scope_generation()
                || workspace() != "Layout"
                || pending.read().is_some()
                || pending_preset.read().is_some()
                || pending_delete.read().is_some()
                || preparing_preset()
                || duplicating()
            {
                return;
            }
            let selected = selected_context.read().clone();
            let model = runtime.model();
            let scope = runtime.scope();
            let Some((projection, editable)) = project_current_for(
                &runtime,
                &model,
                scope.as_ref(),
                selected.as_ref(),
                MatrixProjectionContext {
                    editor_instance_id,
                    context_generation: context_generation.borrow().value,
                    scope_generation: scope_generation(),
                    workspace: workspace(),
                },
            ) else {
                return;
            };
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if projection.owner != request.owner
                || !editable
                || snapshot.token != request.snapshot_token
                || snapshot.document.revision != request.revision
            {
                runtime.report("The selected matrix changed. Reopen the inspector before duplicating the design.");
                return;
            }
            let original = (*snapshot.document).clone();
            let report_scope = Rc::new(RefCell::new(Some(VariantReportOwner {
                scope: request.owner.scope.clone(),
                token: request.snapshot_token,
                revision: request.revision,
            })));
            duplicating.set(true);
            let runtime = runtime.clone();
            let alive = alive.clone();
            let report_scope_for_task = report_scope.clone();
            spawn_local(async move {
                let result = duplicate_design_variant(
                    runtime.clone(),
                    &request,
                    original,
                    alive.clone(),
                    report_scope_for_task.clone(),
                )
                .await;
                if !alive.get() {
                    return;
                }
                duplicating.set(false);
                if let Err(message) = result {
                    let model = runtime.model();
                    let scope = runtime.scope();
                    let still_owned =
                        report_scope_for_task
                            .borrow()
                            .as_ref()
                            .is_some_and(|owner| {
                                scope.as_ref() == Some(&owner.scope)
                                    && model.accepted.is_some_and(|accepted| {
                                        accepted.session_epoch == owner.scope.session_epoch
                                            && accepted.document.id == owner.scope.document_id
                                            && accepted.token == owner.token
                                            && accepted.document.revision == owner.revision
                                    })
                            });
                    if still_owned {
                        runtime.report(message);
                    }
                }
            });
        }
    });

    let on_add_row = use_callback({
        let runtime = runtime.clone();
        let mut request_sequence = request_sequence;
        let context_generation = context_generation.clone();
        move |()| {
            let selected = selected_context.read().clone();
            let model = runtime.model();
            let scope = runtime.scope();
            let Some((projection, editable)) = project_current_for(
                &runtime,
                &model,
                scope.as_ref(),
                selected.as_ref(),
                MatrixProjectionContext {
                    editor_instance_id,
                    context_generation: context_generation.borrow().value,
                    scope_generation: scope_generation(),
                    workspace: workspace(),
                },
            ) else {
                return;
            };
            if !editable || projection.rows == u32::MAX {
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let request_id = request_sequence().saturating_add(1);
            if request_id == request_sequence() {
                return;
            }
            request_sequence.set(request_id);
            on_edit.call(MatrixEditRequest {
                owner: projection.owner,
                request_id,
                snapshot_token: snapshot.token,
                revision: snapshot.document.revision,
                field: MatrixEditField::Rows,
                baseline: MatrixEditValue::Rows(projection.rows),
                value: MatrixEditValue::Rows(projection.rows + 1),
            });
        }
    });

    let on_add_column = use_callback({
        let runtime = runtime.clone();
        let mut request_sequence = request_sequence;
        let context_generation = context_generation.clone();
        move |()| {
            let selected = selected_context.read().clone();
            let model = runtime.model();
            let scope = runtime.scope();
            let Some((projection, editable)) = project_current_for(
                &runtime,
                &model,
                scope.as_ref(),
                selected.as_ref(),
                MatrixProjectionContext {
                    editor_instance_id,
                    context_generation: context_generation.borrow().value,
                    scope_generation: scope_generation(),
                    workspace: workspace(),
                },
            ) else {
                return;
            };
            if !editable || projection.columns == u32::MAX {
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let request_id = request_sequence().saturating_add(1);
            if request_id == request_sequence() {
                return;
            }
            request_sequence.set(request_id);
            on_edit.call(MatrixEditRequest {
                owner: projection.owner,
                request_id,
                snapshot_token: snapshot.token,
                revision: snapshot.document.revision,
                field: MatrixEditField::Columns,
                baseline: MatrixEditValue::Columns(projection.columns),
                value: MatrixEditValue::Columns(projection.columns + 1),
            });
        }
    });

    let on_unlink = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        move |request: MatrixUnlinkRequest| {
            if request.owner.editor_instance_id != editor_instance_id
                || request.owner.context_generation != context_generation.borrow().value
                || request.owner.scope_generation != scope_generation()
                || workspace() != "Layout"
                || pending.read().is_some()
                || pending_preset.read().is_some()
                || pending_delete.read().is_some()
                || preparing_preset()
                || duplicating()
            {
                return;
            }
            let selected = selected_context.read().clone();
            let model = runtime.model();
            let scope = runtime.scope();
            let Some((projection, editable)) = project_current_for(
                &runtime,
                &model,
                scope.as_ref(),
                selected.as_ref(),
                MatrixProjectionContext {
                    editor_instance_id,
                    context_generation: context_generation.borrow().value,
                    scope_generation: scope_generation(),
                    workspace: workspace(),
                },
            ) else {
                return;
            };
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if projection.owner != request.owner
                || !editable
                || snapshot.token != request.snapshot_token
                || snapshot.document.revision != request.revision
                || projection
                    .layout_relation
                    .as_ref()
                    .and_then(|relation| relation.unlink_layout_id.as_deref())
                    != Some(request.layout_id.as_str())
            {
                runtime.report(
                    "The selected layout relationship changed. Reopen the inspector before unlinking the halves.",
                );
                return;
            }
            let Some(mut layout) = snapshot
                .document
                .layouts
                .iter()
                .find(|layout| {
                    layout.id == request.layout_id
                        && layout.board_id == request.owner.scope.board_id
                })
                .cloned()
            else {
                return;
            };
            if layout.mirror_link.is_none() {
                return;
            }
            layout.mirror_link = None;
            let operation_id = runtime.operation();
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id: format!("matrix-inspector-unlink-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![layout.id.clone()],
                    operation: EditOperation::SetLayout { layout },
                },
            });
        }
    });

    MatrixInspectorMount {
        projection,
        request_sequence,
        editable,
        busy: pending.read().is_some()
            || pending_preset.read().is_some()
            || pending_delete.read().is_some()
            || preparing_preset()
            || duplicating(),
        feedback: feedback.read().clone(),
        on_edit,
        on_add_row,
        on_add_column,
        on_apply_preset,
        on_delete,
        on_unlink,
        on_duplicate,
    }
}

fn project_current(
    runtime: &Runtime,
    selected: Option<&ScopedTreeContext>,
    editor_instance_id: u64,
    context_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
) -> (Option<MatrixInspectorProjection>, bool) {
    if workspace != "Layout" {
        return (None, false);
    }
    let model = runtime.model();
    let scope = runtime.scope();
    project_current_for(
        runtime,
        &model,
        scope.as_ref(),
        selected,
        MatrixProjectionContext {
            editor_instance_id,
            context_generation,
            scope_generation,
            workspace,
        },
    )
    .map_or((None, false), |(projection, editable)| {
        (Some(projection), editable)
    })
}

struct MatrixProjectionContext {
    editor_instance_id: u64,
    context_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
}

pub(super) fn switch_choices(
    document: &ProjectDoc,
    templates: &[PartDefinition],
    current_id: &str,
) -> Vec<(String, String)> {
    let mut definitions = Vec::<PartDefinition>::new();
    for definition in templates.iter().chain(&document.definitions) {
        if let Some(existing) = definitions
            .iter_mut()
            .find(|existing| existing.id == definition.id)
        {
            *existing = definition.clone();
        } else {
            definitions.push(definition.clone());
        }
    }
    if !definitions
        .iter()
        .any(|definition| definition.id == current_id)
    {
        definitions.push(PartDefinition {
            id: current_id.into(),
            name: current_id.into(),
            kind: boardstudio_core::model::PartKind::Switch,
            hardware_profile: None,
            input_profile: None,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: Vec::new(),
            pads: Vec::new(),
            models: None,
            generator: None,
            mechanical_profile: None,
        });
    }
    definitions
        .into_iter()
        .filter(|definition| {
            definition.id == current_id
                || (!is_assembly_snapshot(&definition.id, definition.kicad_source.is_some())
                    && matrix_input_available(definition))
        })
        .map(|definition| {
            let label = match definition.id.as_str() {
                "generator:ceoloide/switch_mx" => "MX switch".into(),
                "generator:ceoloide/switch_choc_v1_v2" => "Choc V1 / V2 switch".into(),
                "generator:ceoloide/switch_gateron_ks27_ks33" => "Gateron KS27 / KS33 switch".into(),
                _ => definition.name.clone(),
            };
            (definition.id, label)
        })
        .collect()
}

fn is_assembly_snapshot(id: &str, has_kicad_source: bool) -> bool {
    if has_kicad_source {
        return false;
    }
    let Some((prefix, _)) = id.split_once("/definition/") else {
        return false;
    };
    prefix.starts_with("assembly-")
        || (prefix.len() == 36
            && prefix.bytes().enumerate().all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                _ => byte.is_ascii_hexdigit(),
            }))
}

fn parse_preset_variant(
    variant: Option<&str>,
) -> (Option<MatrixPreset>, Option<SwitchOrientation>) {
    let Some(variant) = variant.and_then(|value| value.strip_prefix("preset/")) else {
        return (None, None);
    };
    let mut segments = variant.split('/');
    let preset = segments.next().and_then(parse_matrix_preset);
    let orientation = variant.rsplit('/').next().and_then(|value| match value {
        "north" => Some(SwitchOrientation::North),
        "south" => Some(SwitchOrientation::South),
        _ => None,
    });
    (preset, orientation)
}

fn parse_matrix_preset(value: &str) -> Option<MatrixPreset> {
    Some(match value {
        "mx-solder" => MatrixPreset::MxSolder,
        "mx-hotswap" => MatrixPreset::MxHotswap,
        "choc-solder" => MatrixPreset::ChocSolder,
        "choc-hotswap" => MatrixPreset::ChocHotswap,
        "mx-rgb" => MatrixPreset::MxRgb,
        "choc-rgb" => MatrixPreset::ChocRgb,
        "mx-hotswap-rgb" => MatrixPreset::MxHotswapRgb,
        "choc-hotswap-rgb" => MatrixPreset::ChocHotswapRgb,
        _ => return None,
    })
}

fn matrix_preset_variant(matrix: &Matrix) -> Option<String> {
    matrix
        .cells
        .iter()
        .find(|cell| cell.definition_id.as_deref() == Some(matrix.definition_id.as_str()))
        .and_then(|cell| cell.variant.clone())
}

fn preset_variant(
    preset: MatrixPreset,
    orientation: SwitchOrientation,
    reversible: bool,
) -> String {
    if reversible {
        format!(
            "preset/{}/reversible/{}",
            preset.as_str(),
            orientation.as_str()
        )
    } else {
        format!("preset/{}/{}", preset.as_str(), orientation.as_str())
    }
}

fn matrix_input_available(definition: &PartDefinition) -> bool {
    let press = if let Some(profile) = definition.input_profile.as_ref() {
        profile
            .press
            .as_ref()
            .map(|press| (press.row.as_str(), press.column.as_str(), press.independent))
    } else if let Some(press) = definition.matrix_terminals.as_ref() {
        Some((press.row.as_str(), press.column.as_str(), true))
    } else if definition
        .generator
        .as_ref()
        .is_some_and(|generator| generator.source == "ceoloide/rotary_encoder_ec11_ec12")
    {
        Some(("S1", "S2", true))
    } else {
        None
    };
    let Some((row, column, independent)) = press else {
        return definition.input_profile.is_none()
            && definition.pads.iter().any(|pad| pad.id == "one")
            && definition.pads.iter().any(|pad| pad.id == "two");
    };
    if !independent {
        return false;
    }
    let pads_for = |terminal: &str| {
        definition
            .terminals
            .get(terminal)
            .cloned()
            .unwrap_or_else(|| {
                definition
                    .pads
                    .iter()
                    .filter(|pad| pad.id == terminal || pad.number == terminal)
                    .map(|pad| pad.id.clone())
                    .collect()
            })
    };
    let row_pads = pads_for(row);
    let column_pads = pads_for(column);
    if row_pads.is_empty()
        || column_pads.is_empty()
        || row_pads.iter().any(|pad| column_pads.contains(pad))
        || row_pads
            .iter()
            .chain(&column_pads)
            .any(|id| !definition.pads.iter().any(|pad| &pad.id == id))
    {
        return false;
    }
    let rotary_pads: Vec<_> = definition
        .input_profile
        .as_ref()
        .and_then(|profile| profile.rotary.as_ref())
        .into_iter()
        .flat_map(|rotary| [&rotary.a, &rotary.b, &rotary.common])
        .flat_map(|terminal| pads_for(terminal))
        .collect();
    !row_pads
        .iter()
        .chain(&column_pads)
        .any(|id| rotary_pads.contains(id))
}

fn project_current_for(
    runtime: &Runtime,
    model: &boardstudio_application::ReadModel,
    live_scope: Option<&Scope>,
    selected: Option<&ScopedTreeContext>,
    context: MatrixProjectionContext,
) -> Option<(MatrixInspectorProjection, bool)> {
    let MatrixProjectionContext {
        editor_instance_id,
        context_generation,
        scope_generation,
        workspace,
    } = context;
    if workspace != "Layout" {
        return None;
    }
    let scope = live_scope?;
    let selected = selected?;
    if &selected.scope != scope || runtime.scope().as_ref() != Some(scope) {
        return None;
    }
    let TreeContext::Matrix { matrix_id } = &selected.context else {
        return None;
    };
    let snapshot = model.accepted.as_ref()?;
    if snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
        || super::resolve_selection(model, &selected.context).is_none()
    {
        return None;
    }
    let matrix = snapshot
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == *matrix_id)?;
    let layout = snapshot
        .document
        .layouts
        .iter()
        .find(|layout| layout.board_id == scope.board_id && layout.matrix_id == matrix.id);
    let (name_target, name_label, name_value, name_baseline) = if let Some(layout) = layout {
        (
            MatrixNameTarget::Layout {
                id: layout.id.clone(),
            },
            "Layout name",
            layout.name.clone(),
            MatrixEditValue::Name(Some(layout.name.clone())),
        )
    } else {
        let name = matrix.name.clone();
        let fallback = super::context_label(model, &selected.context)
            .unwrap_or_else(|| format!("Matrix {}", matrix.id));
        (
            MatrixNameTarget::Matrix,
            "Matrix name",
            name.clone().unwrap_or(fallback),
            MatrixEditValue::Name(name),
        )
    };
    let saved = model.lifecycle == Lifecycle::Ready
        && model.durability
            == (Durability::Saved {
                revision: snapshot.document.revision,
            });
    let editable = saved && model.display_preview.is_none() && model.gesture.is_none();
    let baseline_variant = matrix
        .cells
        .iter()
        .find(|cell| cell.definition_id.as_deref() == Some(matrix.definition_id.as_str()))
        .and_then(|cell| cell.variant.clone());
    let (preset, orientation) = parse_preset_variant(baseline_variant.as_deref());
    let unlink_layout = layout.and_then(|active_layout| {
        if active_layout.mirror_link.is_some() {
            Some(active_layout)
        } else {
            snapshot.document.layouts.iter().find(|candidate| {
                candidate.board_id == scope.board_id
                    && candidate
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == active_layout.id)
            })
        }
    });
    let partner_layout = unlink_layout.and_then(|linked_layout| {
        if layout.is_some_and(|active_layout| active_layout.id == linked_layout.id) {
            linked_layout.mirror_link.as_ref().and_then(|link| {
                snapshot.document.layouts.iter().find(|candidate| {
                    candidate.board_id == scope.board_id && candidate.id == link.source_id
                })
            })
        } else {
            Some(linked_layout)
        }
    });
    let layout_relation = layout.map(|_| MatrixLayoutRelation {
        partner_name: partner_layout.map(|partner| partner.name.clone()),
        unlink_layout_id: unlink_layout
            .filter(|linked_layout| linked_layout.mirror_link.is_some())
            .map(|linked_layout| linked_layout.id.clone()),
    });
    Some((
        MatrixInspectorProjection {
            owner: MatrixInspectorOwner {
                editor_instance_id,
                context_generation,
                scope_generation,
                scope: scope.clone(),
                matrix_id: matrix.id.clone(),
                name_target,
            },
            snapshot_token: snapshot.token,
            revision: snapshot.document.revision,
            matrix_label: super::context_label(model, &selected.context)
                .unwrap_or_else(|| format!("Matrix {}", matrix.id)),
            name_label,
            name_value,
            name_baseline,
            rows: matrix.rows,
            columns: matrix.columns,
            pitch_x: matrix.pitch.x,
            pitch_y: matrix.pitch.y,
            definition_id: matrix.definition_id.clone(),
            switch_choices: switch_choices(&snapshot.document, &[], &matrix.definition_id),
            diode_direction: matrix.diode_direction.unwrap_or(DiodeDirection::Row2col),
            edge_gap_x: matrix.edge_gap.as_ref().map_or(1.0, |gap| gap.x),
            edge_gap_y: matrix.edge_gap.as_ref().map_or(1.0, |gap| gap.y),
            preset,
            orientation,
            baseline_variant,
            layout_relation,
        },
        editable,
    ))
}

fn settle_pending(
    runtime: &Runtime,
    scope_generation: u64,
    pending: &mut Signal<Option<PendingMatrixEdit>>,
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
) {
    let Some(waiting) = pending.read().clone() else {
        return;
    };
    let model = runtime.model();
    let scope = runtime.scope();
    if scope.as_ref() != Some(&waiting.request.owner.scope)
        || scope_generation != waiting.request.owner.scope_generation
    {
        pending.set(None);
        feedback.set(Vec::new());
        return;
    }
    let Some(outcome) = waiting.outcome.borrow().clone() else {
        return;
    };
    match outcome {
        TerminalOutcome::Completed => {
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if matches!(model.durability, Durability::Failed { .. })
                || matches!(
                    model.lifecycle,
                    Lifecycle::RecoveryRequired | Lifecycle::Closed
                )
            {
                finish_pending(
                    pending,
                    feedback,
                    &waiting,
                    MatrixEditState::Failed,
                    Some("The matrix operation completed, but the accepted document did not save. Retry after recovery.".into()),
                );
                return;
            }
            if snapshot.token == waiting.base_token
                || snapshot.document.revision <= waiting.base_revision
                || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: snapshot.document.revision,
                    })
            {
                return;
            }
            let value = field_value(
                &snapshot.document,
                &waiting.request.owner.matrix_id,
                &waiting.request.owner.name_target,
                waiting.request.field,
            );
            if value.as_ref() == Some(&waiting.request.value) {
                finish_pending(pending, feedback, &waiting, MatrixEditState::Saved, None);
            } else {
                finish_pending(
                    pending,
                    feedback,
                    &waiting,
                    MatrixEditState::Failed,
                    Some("The saved matrix does not contain the requested value. Review the current field and retry.".into()),
                );
            }
        }
        TerminalOutcome::Rejected(message)
        | TerminalOutcome::PersistenceFailed(message)
        | TerminalOutcome::BlockedByRecovery(message)
        | TerminalOutcome::ExecutorFailed(message) => finish_pending(
            pending,
            feedback,
            &waiting,
            MatrixEditState::Failed,
            Some(message),
        ),
        TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
            finish_pending(
                pending,
                feedback,
                &waiting,
                MatrixEditState::Failed,
                Some("The matrix edit did not complete in the active session. Review the current value and retry.".into()),
            );
        }
    }
}

fn settle_pending_preset(
    runtime: &Runtime,
    scope_generation: u64,
    pending: &mut Signal<Option<PendingMatrixPreset>>,
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
) {
    let Some(waiting) = pending.read().clone() else {
        return;
    };
    if runtime.scope().as_ref() != Some(&waiting.request.owner.scope)
        || scope_generation != waiting.request.owner.scope_generation
    {
        pending.set(None);
        return;
    }
    let Some(outcome) = waiting.outcome.borrow().clone() else {
        return;
    };
    match outcome {
        TerminalOutcome::Completed => {
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if matches!(model.durability, Durability::Failed { .. })
                || matches!(
                    model.lifecycle,
                    Lifecycle::RecoveryRequired | Lifecycle::Closed
                )
            {
                finish_action(
                    pending,
                    feedback,
                    &waiting,
                    MatrixEditState::Failed,
                    Some("The preset edit completed, but the project did not save. Recover the project and retry.".into()),
                );
                return;
            }
            if snapshot.token == waiting.base_token
                || snapshot.document.revision <= waiting.base_revision
                || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: snapshot.document.revision,
                    })
            {
                return;
            }
            let actual = snapshot
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == waiting.request.owner.matrix_id)
                .and_then(matrix_preset_variant);
            if actual.as_deref() == Some(waiting.expected_variant.as_str()) {
                finish_action(pending, feedback, &waiting, MatrixEditState::Saved, None);
            } else {
                finish_action(
                    pending,
                    feedback,
                    &waiting,
                    MatrixEditState::Failed,
                    Some("The saved matrix does not contain the requested preset. Review it and retry.".into()),
                );
            }
        }
        TerminalOutcome::Rejected(message)
        | TerminalOutcome::PersistenceFailed(message)
        | TerminalOutcome::BlockedByRecovery(message)
        | TerminalOutcome::ExecutorFailed(message) => finish_action(
            pending,
            feedback,
            &waiting,
            MatrixEditState::Failed,
            Some(message),
        ),
        TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
            finish_action(
                pending,
                feedback,
                &waiting,
                MatrixEditState::Failed,
                Some("The preset edit did not complete in the active session.".into()),
            )
        }
    }
}

fn settle_pending_delete(
    runtime: &Rc<Runtime>,
    scope_generation: u64,
    pending: &mut Signal<Option<PendingMatrixDelete>>,
    selected_context: Signal<Option<ScopedTreeContext>>,
) {
    let Some(waiting) = pending.read().clone() else {
        return;
    };
    if runtime.scope().as_ref() != Some(&waiting.request.owner.scope)
        || scope_generation != waiting.request.owner.scope_generation
    {
        pending.set(None);
        return;
    }
    let Some(outcome) = waiting.outcome.borrow().clone() else {
        return;
    };
    match outcome {
        TerminalOutcome::Completed => {
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if snapshot.token == waiting.base_token
                || snapshot.document.revision <= waiting.base_revision
                || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: snapshot.document.revision,
                    })
            {
                return;
            }
            if snapshot
                .document
                .matrices
                .iter()
                .any(|matrix| matrix.id == waiting.request.owner.matrix_id)
            {
                pending.set(None);
                runtime.report("The matrix delete completed without removing the selected matrix.");
                return;
            }
            let mut selected_context = selected_context;
            if selected_context.read().as_ref().is_some_and(|selected| {
                selected.scope == waiting.request.owner.scope
                    && matches!(
                        &selected.context,
                        TreeContext::Matrix { matrix_id }
                            if matrix_id == &waiting.request.owner.matrix_id
                    )
            }) {
                selected_context.set(None);
                runtime.submit(Event::SelectParts {
                    operation_id: runtime.operation(),
                    part_ids: Vec::new(),
                    range_part_ids: Vec::new(),
                    mode: boardstudio_application::SelectionMode::Replace,
                });
            }
            pending.set(None);
        }
        TerminalOutcome::Rejected(message)
        | TerminalOutcome::PersistenceFailed(message)
        | TerminalOutcome::BlockedByRecovery(message)
        | TerminalOutcome::ExecutorFailed(message) => {
            pending.set(None);
            runtime.report(format!("Could not delete matrix: {message}"));
        }
        TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
            pending.set(None);
            runtime.report("The matrix delete did not complete in the active project.");
        }
    }
}

fn finish_action(
    pending: &mut Signal<Option<PendingMatrixPreset>>,
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
    waiting: &PendingMatrixPreset,
    state: MatrixEditState,
    message: Option<String>,
) {
    if pending
        .read()
        .as_ref()
        .is_some_and(|current| current.operation_id == waiting.operation_id)
    {
        pending.set(None);
        publish_action_feedback(
            feedback,
            &waiting.request.owner,
            waiting.request.request_id,
            MatrixEditField::ApplyPreset,
            state,
            message,
        );
    }
}

fn publish_action_feedback(
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
    owner: &MatrixInspectorOwner,
    request_id: u64,
    field: MatrixEditField,
    state: MatrixEditState,
    message: Option<String>,
) {
    let next = MatrixEditFeedback {
        owner: owner.clone(),
        request_id,
        field,
        state,
        message,
    };
    let mut entries = feedback.read().clone();
    if let Some(existing) = entries
        .iter_mut()
        .find(|entry| entry.owner == next.owner && entry.request_id == next.request_id)
    {
        *existing = next;
    } else {
        entries.push(next);
    }
    while entries.len() > 8 {
        let removable = entries
            .iter()
            .position(|entry| entry.state != MatrixEditState::Pending);
        let Some(index) = removable else {
            break;
        };
        entries.remove(index);
    }
    feedback.set(entries);
}

fn finish_pending(
    pending: &mut Signal<Option<PendingMatrixEdit>>,
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
    waiting: &PendingMatrixEdit,
    state: MatrixEditState,
    message: Option<String>,
) {
    if pending
        .read()
        .as_ref()
        .is_some_and(|current| current.operation_id == waiting.operation_id)
    {
        pending.set(None);
        publish_feedback(feedback, &waiting.request, state, message);
    }
}

fn publish_feedback(
    feedback: &mut Signal<Vec<MatrixEditFeedback>>,
    request: &MatrixEditRequest,
    state: MatrixEditState,
    message: Option<String>,
) {
    let next = MatrixEditFeedback {
        owner: request.owner.clone(),
        request_id: request.request_id,
        field: request.field,
        state,
        message,
    };
    let mut entries = feedback.read().clone();
    if let Some(existing) = entries
        .iter_mut()
        .find(|entry| entry.owner == next.owner && entry.request_id == next.request_id)
    {
        *existing = next;
    } else {
        entries.push(next);
    }
    while entries.len() > 8 {
        let removable = entries
            .iter()
            .position(|entry| entry.state != MatrixEditState::Pending);
        let Some(index) = removable else {
            break;
        };
        entries.remove(index);
    }
    feedback.set(entries);
}

fn field_value(
    document: &ProjectDoc,
    matrix_id: &str,
    name_target: &MatrixNameTarget,
    field: MatrixEditField,
) -> Option<MatrixEditValue> {
    match field {
        MatrixEditField::Name => match name_target {
            MatrixNameTarget::Matrix => document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .map(|matrix| MatrixEditValue::Name(matrix.name.clone())),
            MatrixNameTarget::Layout { id } => document
                .layouts
                .iter()
                .find(|layout| layout.id == *id && layout.matrix_id == matrix_id)
                .map(|layout| MatrixEditValue::Name(Some(layout.name.clone()))),
        },
        MatrixEditField::Rows => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::Rows(matrix.rows)),
        MatrixEditField::Columns => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::Columns(matrix.columns)),
        MatrixEditField::PitchX => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::PitchX(matrix.pitch.x)),
        MatrixEditField::PitchY => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::PitchY(matrix.pitch.y)),
        MatrixEditField::SwitchDefinition => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::SwitchDefinition(matrix.definition_id.clone())),
        MatrixEditField::DiodeDirection => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| {
                MatrixEditValue::DiodeDirection(
                    matrix.diode_direction.unwrap_or(DiodeDirection::Row2col),
                )
            }),
        MatrixEditField::EdgeGapX => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| {
                MatrixEditValue::EdgeGapX(matrix.edge_gap.as_ref().map_or(1.0, |gap| gap.x))
            }),
        MatrixEditField::EdgeGapY => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| {
                MatrixEditValue::EdgeGapY(matrix.edge_gap.as_ref().map_or(1.0, |gap| gap.y))
            }),
        MatrixEditField::ApplyPreset => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::Name(matrix_preset_variant(matrix))),
    }
}

fn build_operation(
    document: &ProjectDoc,
    matrix: &Matrix,
    name_target: &MatrixNameTarget,
    field: MatrixEditField,
    value: MatrixEditValue,
) -> Result<EditOperation, String> {
    if field == MatrixEditField::Name {
        let MatrixEditValue::Name(Some(name)) = value else {
            return Err("Enter a non-empty matrix or layout name.".into());
        };
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a non-empty matrix or layout name.".into());
        }
        return match name_target {
            MatrixNameTarget::Matrix => {
                let mut replacement = matrix.clone();
                replacement.name = Some(name.to_owned());
                Ok(EditOperation::SetMatrix {
                    matrix: replacement,
                    definitions: None,
                })
            }
            MatrixNameTarget::Layout { id } => {
                let mut layout = document
                    .layouts
                    .iter()
                    .find(|layout| layout.id == *id && layout.matrix_id == matrix.id)
                    .cloned()
                    .ok_or_else(|| "The selected layout no longer owns this matrix.".to_owned())?;
                layout.name = name.to_owned();
                Ok(EditOperation::SetLayout { layout })
            }
        };
    }
    if matches!(name_target, MatrixNameTarget::Layout { .. }) && field == MatrixEditField::Name {
        return Err("The layout name target is unavailable.".into());
    }
    let replacement = matrix_edit(matrix, field, value)?;
    Ok(EditOperation::SetMatrix {
        matrix: replacement,
        definitions: None,
    })
}

/// Apply one real matrix field edit to a fresh accepted matrix. Row/column edits use the exact
/// presentation request recipe from the existing React `matrixResize.ts`; Core still validates
/// and applies the replacement through `SetMatrix`.
fn matrix_edit(
    matrix: &Matrix,
    field: MatrixEditField,
    value: MatrixEditValue,
) -> Result<Matrix, String> {
    match (field, value) {
        (MatrixEditField::Rows, MatrixEditValue::Rows(rows)) => {
            resize_matrix(matrix, rows, matrix.columns)
        }
        (MatrixEditField::Columns, MatrixEditValue::Columns(columns)) => {
            resize_matrix(matrix, matrix.rows, columns)
        }
        (MatrixEditField::PitchX, MatrixEditValue::PitchX(x)) if x.is_finite() && x > 0.0 => {
            let mut next = matrix.clone();
            next.pitch.x = x;
            Ok(next)
        }
        (MatrixEditField::PitchY, MatrixEditValue::PitchY(y)) if y.is_finite() && y > 0.0 => {
            let mut next = matrix.clone();
            next.pitch.y = y;
            Ok(next)
        }
        (MatrixEditField::SwitchDefinition, MatrixEditValue::SwitchDefinition(id))
            if !id.trim().is_empty() =>
        {
            let mut next = matrix.clone();
            next.definition_id = id;
            Ok(next)
        }
        (MatrixEditField::DiodeDirection, MatrixEditValue::DiodeDirection(direction)) => {
            let mut next = matrix.clone();
            next.diode_direction = Some(direction);
            Ok(next)
        }
        (MatrixEditField::EdgeGapX, MatrixEditValue::EdgeGapX(x)) if x.is_finite() && x >= 0.0 => {
            let mut next = matrix.clone();
            let mut gap = next.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
            gap.x = x;
            next.edge_gap = Some(gap);
            Ok(next)
        }
        (MatrixEditField::EdgeGapY, MatrixEditValue::EdgeGapY(y)) if y.is_finite() && y >= 0.0 => {
            let mut next = matrix.clone();
            let mut gap = next.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
            gap.y = y;
            next.edge_gap = Some(gap);
            Ok(next)
        }
        _ => Err("The matrix field value is invalid.".into()),
    }
}

/// Build the accepted matrix resize request.
fn resize_matrix(matrix: &Matrix, rows: u32, columns: u32) -> Result<Matrix, String> {
    if rows == 0 || columns == 0 {
        return Err("Matrix dimensions must be positive whole numbers.".into());
    }
    if rows
        .checked_mul(columns)
        .is_none_or(|cell_count| cell_count > 4096)
    {
        return Err("Matrix dimensions cannot exceed 4096 cells.".into());
    }
    let template = matrix.cells.iter().find(|cell| {
        cell.definition_id.as_deref() == Some(matrix.definition_id.as_str())
            && cell
                .variant
                .as_deref()
                .is_some_and(|variant| variant.starts_with("preset/"))
    });
    let mut replacement = matrix.clone();
    replacement.rows = rows;
    replacement.columns = columns;
    let Some(template) = template else {
        return Ok(replacement);
    };

    replacement
        .cells
        .retain(|cell| cell.row < rows && cell.column < columns);
    let assemblies: Vec<MatrixAssembly> = template
        .assemblies
        .iter()
        .filter(|assembly| assembly.id == "diode" || assembly.id == "led")
        .cloned()
        .collect();
    let definition_id = template.definition_id.clone();
    let variant = template.variant.clone();
    let assemblies_local = template.assemblies_local;
    let rotation = Some(
        if variant
            .as_deref()
            .is_some_and(|variant| variant.ends_with("/north"))
        {
            180.0
        } else {
            0.0
        },
    );
    for row in 0..rows {
        for column in 0..columns {
            if row < matrix.rows && column < matrix.columns {
                continue;
            }
            replacement.cells.push(MatrixCell {
                row,
                column,
                enabled: true,
                definition_id: definition_id.clone(),
                variant: variant.clone(),
                offset: None,
                rotation,
                assemblies: assemblies.clone(),
                assemblies_local,
            });
        }
    }
    Ok(replacement)
}

async fn prepare_preset_matrix(
    document: &ProjectDoc,
    matrix: &Matrix,
    board_id: &str,
    preset: MatrixPreset,
    orientation: SwitchOrientation,
    reversible: bool,
) -> Result<(Matrix, Vec<PartDefinition>), String> {
    let templates = crate::presentation::parts::load_matrix_templates(reversible).await?;
    let setup_preset = matrix_setup_preset(preset);
    let mut prepared = crate::matrix_setup_operation::prepare_matrix(
        matrix.id.clone(),
        board_id.to_owned(),
        crate::matrix_setup_operation::MatrixSetupRequest {
            rows: matrix.rows,
            columns: matrix.columns,
            preset: setup_preset,
        },
        reversible,
        &templates,
    )?;
    for definition in &mut prepared.definitions {
        *definition =
            crate::presentation::parts::normalize_matrix_definition(definition.clone()).await?;
        if document
            .definitions
            .iter()
            .any(|existing| existing.id == definition.id && existing != definition)
        {
            return Err(format!(
                "The prepared matrix definition {} conflicts with an existing definition.",
                definition.id
            ));
        }
    }
    let variant = preset_variant(preset, orientation, reversible);
    let replacement = matrix_with_preset(matrix, &prepared.matrix, &variant, orientation);
    Ok((replacement, prepared.definitions))
}

/// Clone the whole accepted project, open it through the Session lifecycle, and apply the chosen
/// assembly preset to the copy. If preset preparation or application fails, reopen the saved
/// source project so the user returns to the design they started from.
#[allow(clippy::too_many_arguments)]
async fn duplicate_design_variant(
    runtime: Rc<Runtime>,
    request: &MatrixDuplicateRequest,
    mut document: ProjectDoc,
    alive: Rc<Cell<bool>>,
    report_scope: Rc<RefCell<Option<VariantReportOwner>>>,
) -> Result<(), String> {
    if !alive.get() {
        return Err("The matrix design variant owner closed.".into());
    }
    let source_session_epoch = request.owner.scope.session_epoch;
    let variant_session_epoch = boardstudio_application::SessionEpoch(
        source_session_epoch
            .0
            .checked_add(1)
            .ok_or_else(|| "The project session identity is exhausted.".to_owned())?,
    );
    document.id = crate::runtime::new_project_id()?;
    let source_name = document.name.trim();
    let source_name = if source_name.is_empty() {
        "Untitled keyboard"
    } else {
        source_name
    };
    document.name = format!("{source_name} (variant)");
    let variant_id = document.id.clone();
    let open_id = runtime.operation();
    let open_outcome = runtime.observe_operation(open_id);
    let open_event = if runtime.model().lifecycle == Lifecycle::RecoveryRequired {
        Event::RecoverWithDocument {
            operation_id: open_id,
            document,
        }
    } else {
        Event::Open {
            operation_id: open_id,
            document,
        }
    };
    runtime.submit(open_event);
    let accepted = match wait_for_matrix_project(
        &runtime,
        open_outcome,
        &variant_id,
        variant_session_epoch,
        &alive,
    )
    .await
    {
        Ok(accepted) => accepted,
        Err(error) => {
            if let (Some(active), Some(scope)) = (runtime.model().accepted, runtime.scope())
                && active.session_epoch == variant_session_epoch
                && active.document.id == variant_id
                && scope.session_epoch == variant_session_epoch
                && scope.document_id == variant_id
            {
                *report_scope.borrow_mut() = Some(VariantReportOwner {
                    scope,
                    token: active.token,
                    revision: active.document.revision,
                });
            }
            let recovery = if alive.get() {
                restore_source_after_variant_failure(
                    &runtime,
                    request,
                    &variant_id,
                    variant_session_epoch,
                    None,
                    &report_scope,
                )
                .await
            } else {
                Ok(false)
            };
            return Err(match recovery {
                Ok(true) => format!(
                    "Could not open the matrix design variant; reopened the saved original. {error}"
                ),
                Ok(false) => format!("Could not open the matrix design variant. {error}"),
                Err(recovery_error) => format!(
                    "Could not open the matrix design variant ({error}); could not confirm reopening the saved original ({recovery_error})."
                ),
            });
        }
    };
    if !alive.get() {
        return Err("The matrix design variant owner closed.".into());
    }
    let accepted = match navigate_variant_to_source_context(
        &runtime,
        accepted,
        &variant_id,
        &request.owner.scope,
        &alive,
    )
    .await
    {
        Ok(accepted) => accepted,
        Err(error) => {
            let recovery = if alive.get() {
                restore_source_after_variant_failure(
                    &runtime,
                    request,
                    &variant_id,
                    variant_session_epoch,
                    None,
                    &report_scope,
                )
                .await
            } else {
                Ok(false)
            };
            return Err(match recovery {
                Ok(true) => format!(
                    "Could not select the original board context; reopened the saved original. {error}"
                ),
                Ok(false) => format!("Could not select the original board context. {error}"),
                Err(recovery_error) => format!(
                    "Could not select the original board context ({error}); could not confirm reopening the saved original ({recovery_error})."
                ),
            });
        }
    };
    let clone_scope = runtime
        .scope()
        .ok_or_else(|| "The duplicated project has no active scope.".to_owned())?;
    *report_scope.borrow_mut() = Some(VariantReportOwner {
        scope: clone_scope,
        token: accepted.token,
        revision: accepted.document.revision,
    });
    let reversible = is_reversible(&accepted.document);
    let result = async {
        let accepted =
            exact_variant_snapshot(&runtime, &accepted, &variant_id, &request.owner.scope)?;
        let matrix = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == request.owner.matrix_id)
            .cloned()
            .ok_or_else(|| {
                "The copied project no longer contains the selected matrix.".to_owned()
            })?;
        let (replacement, definitions) = prepare_preset_matrix(
            &accepted.document,
            &matrix,
            &request.owner.scope.board_id,
            request.preset,
            request.orientation,
            reversible,
        )
        .await?;
        if !alive.get() {
            return Err("The matrix design variant owner closed.".into());
        }
        // Preparing the catalogue can take long enough for the user to open another project.
        // Re-read the exact accepted clone before submitting any operation into Session.
        let current =
            exact_variant_snapshot(&runtime, &accepted, &variant_id, &request.owner.scope)?;
        let operation_id = runtime.operation();
        let outcome = runtime.observe_operation(operation_id);
        runtime.submit(Event::Edit {
            operation_id,
            command: EditCommand {
                base_revision: current.document.revision,
                transaction_id: format!("matrix-duplicate-variant-{}", operation_id.0),
                phase: EditPhase::Commit,
                target_ids: vec![matrix.id.clone()],
                operation: EditOperation::SetMatrix {
                    matrix: replacement,
                    definitions: Some(definitions),
                },
            },
        });
        wait_for_variant_edit(
            &runtime,
            outcome,
            &current,
            &variant_id,
            &request.owner.scope,
            &matrix.id,
            &preset_variant(request.preset, request.orientation, reversible),
            &alive,
        )
        .await?;
        Ok::<(), String>(())
    }
    .await;
    if let Err(error) = result {
        let recovery = if alive.get() {
            restore_source_after_variant_failure(
                &runtime,
                request,
                &variant_id,
                variant_session_epoch,
                Some(&accepted),
                &report_scope,
            )
            .await
        } else {
            Ok(false)
        };
        return Err(match recovery {
            Ok(true) => format!(
                "Could not prepare the matrix design variant; reopened the saved original. {error}"
            ),
            Ok(false) => format!("Could not prepare the matrix design variant. {error}"),
            Err(recovery_error) => format!(
                "Could not prepare the matrix design variant ({error}); could not confirm reopening the saved original ({recovery_error})."
            ),
        });
    }
    // Confirm the clone remains the active project after its matrix edit was durably saved.
    if runtime
        .model()
        .accepted
        .is_none_or(|active| active.document.id != variant_id)
    {
        return Err("The duplicated project was superseded by another open action.".into());
    }
    Ok(())
}

fn exact_variant_snapshot(
    runtime: &Runtime,
    expected: &boardstudio_application::AcceptedSnapshot,
    variant_id: &str,
    source_scope: &Scope,
) -> Result<boardstudio_application::AcceptedSnapshot, String> {
    let model = runtime.model();
    let Some(current) = model.accepted else {
        return Err("The duplicated project is no longer active.".into());
    };
    let current_scope = runtime.scope();
    if model.lifecycle != Lifecycle::Ready
        || !variant_session_matches(
            expected.session_epoch,
            variant_id,
            current.session_epoch,
            &current.document.id,
        )
        || current.token != expected.token
        || current.document.revision != expected.document.revision
        || !current_scope.is_some_and(|scope| {
            scope.session_epoch == expected.session_epoch
                && scope.document_id == variant_id
                && scope.board_id == source_scope.board_id
                && scope.instance_id == source_scope.instance_id
        })
    {
        return Err("The duplicated project changed before its matrix preset was ready.".into());
    }
    Ok(current)
}

#[allow(clippy::too_many_arguments)]
async fn wait_for_variant_edit(
    runtime: &Rc<Runtime>,
    outcome: OutcomeSlot,
    before: &boardstudio_application::AcceptedSnapshot,
    variant_id: &str,
    source_scope: &Scope,
    matrix_id: &str,
    expected_variant: &str,
    alive: &Rc<Cell<bool>>,
) -> Result<(), String> {
    for _ in 0..1_200 {
        if !alive.get() {
            return Err("The matrix design variant owner closed.".into());
        }
        let model = runtime.model();
        let Some(current) = model.accepted.as_ref() else {
            return Err("The duplicated project is no longer active.".into());
        };
        let current_scope = runtime.scope();
        if !variant_session_matches(
            before.session_epoch,
            variant_id,
            current.session_epoch,
            &current.document.id,
        ) || !current_scope.is_some_and(|scope| {
            scope.session_epoch == before.session_epoch
                && scope.document_id == variant_id
                && scope.board_id == source_scope.board_id
                && scope.instance_id == source_scope.instance_id
        }) {
            return Err("A newer project open superseded the matrix design variant.".into());
        }
        if let Some(outcome) = outcome.borrow_mut().take() {
            if outcome != TerminalOutcome::Completed {
                return Err(format!(
                    "The matrix preset edit did not complete: {outcome:?}"
                ));
            }
            if model.lifecycle != Lifecycle::Ready
                || current.document.revision <= before.document.revision
                || model.durability
                    != (Durability::Saved {
                        revision: current.document.revision,
                    })
            {
                return Err("The copied matrix preset was not durably saved.".into());
            }
            if !current.document.matrices.iter().any(|matrix| {
                matrix.id == matrix_id
                    && matrix.cells.iter().any(|cell| {
                        cell.definition_id.as_deref() == Some(matrix.definition_id.as_str())
                            && cell.variant.as_deref() == Some(expected_variant)
                    })
            }) {
                return Err(
                    "The copied matrix did not retain the requested assembly preset.".into(),
                );
            }
            return Ok(());
        }
        gloo_timers::future::TimeoutFuture::new(25).await;
    }
    Err("The copied matrix preset did not finish saving.".into())
}

async fn restore_source_after_variant_failure(
    runtime: &Rc<Runtime>,
    request: &MatrixDuplicateRequest,
    variant_id: &str,
    variant_session_epoch: boardstudio_application::SessionEpoch,
    owned_clone: Option<&boardstudio_application::AcceptedSnapshot>,
    report_scope: &Rc<RefCell<Option<VariantReportOwner>>>,
) -> Result<bool, String> {
    let source_scope = &request.owner.scope;
    let source_id = source_scope.document_id.as_str();
    let source_session_epoch = source_scope.session_epoch;
    let source_token = request.snapshot_token;
    let source_revision = request.revision;
    let model = runtime.model();
    let Some(current) = model.accepted else {
        return Ok(false);
    };
    let current_scope = runtime.scope();
    let owned_clone_is_current = owned_clone.is_some_and(|owned| {
        variant_session_matches(
            owned.session_epoch,
            variant_id,
            current.session_epoch,
            &current.document.id,
        ) && current_scope.as_ref().is_some_and(|scope| {
            scope.session_epoch == owned.session_epoch
                && scope.document_id == variant_id
                && scope.board_id == source_scope.board_id
                && scope.instance_id == source_scope.instance_id
        }) && current.token == owned.token
            && current.document.revision == owned.document.revision
    });
    let failed_open_is_current = owned_clone.is_none()
        && current.session_epoch == variant_session_epoch
        && current.document.id == variant_id
        && model.lifecycle == Lifecycle::RecoveryRequired;
    let source_needs_recovery = current.session_epoch == source_session_epoch
        && current.document.id == source_id
        && model.lifecycle == Lifecycle::RecoveryRequired
        && current.token == source_token
        && current.document.revision == source_revision
        && current_scope
            .as_ref()
            .is_some_and(|scope| scope == source_scope);
    if !(owned_clone_is_current || failed_open_is_current || source_needs_recovery) {
        return Ok(false);
    }
    let Some(recovery_scope) = current_scope else {
        return Ok(false);
    };
    let restored = runtime
        .reopen_saved_if_current(
            source_id.to_owned(),
            &current,
            model.lifecycle,
            &recovery_scope,
        )
        .await?;
    let restored_scope = runtime
        .scope()
        .filter(|scope| {
            scope.session_epoch == restored.session_epoch
                && scope.document_id == restored.document.id
        })
        .ok_or_else(|| "The saved original reopened without an active project scope.".to_owned())?;
    *report_scope.borrow_mut() = Some(VariantReportOwner {
        scope: restored_scope,
        token: restored.token,
        revision: restored.document.revision,
    });
    Ok(true)
}

async fn navigate_variant_to_source_context(
    runtime: &Rc<Runtime>,
    accepted: boardstudio_application::AcceptedSnapshot,
    variant_id: &str,
    source_scope: &Scope,
    alive: &Rc<Cell<bool>>,
) -> Result<boardstudio_application::AcceptedSnapshot, String> {
    let exact = exact_variant_snapshot(runtime, &accepted, variant_id, source_scope);
    if let Ok(current) = exact {
        return Ok(current);
    }
    let current = runtime
        .model()
        .accepted
        .ok_or_else(|| "The duplicated project is no longer active.".to_owned())?;
    if runtime.model().lifecycle != Lifecycle::Ready
        || current.session_epoch != accepted.session_epoch
        || current.document.id != variant_id
        || current.token != accepted.token
        || current.document.revision != accepted.document.revision
    {
        return Err("The duplicated project changed before its board context was selected.".into());
    }
    if !runtime.scope().is_some_and(|scope| {
        scope.session_epoch == accepted.session_epoch
            && scope.document_id == variant_id
            && scope.board_id == source_scope.board_id
            && scope.instance_id.is_none()
    }) {
        return Err("A newer board navigation superseded matrix variant setup.".into());
    }
    let operation_id = runtime.operation();
    let outcome = runtime.observe_operation(operation_id);
    runtime.submit(Event::Navigate {
        operation_id,
        board_id: source_scope.board_id.clone(),
        instance_id: source_scope.instance_id.clone(),
    });
    for _ in 0..1_200 {
        if !alive.get() {
            return Err("The matrix design variant owner closed.".into());
        }
        let current = runtime
            .model()
            .accepted
            .ok_or_else(|| "The duplicated project is no longer active.".to_owned())?;
        if current.session_epoch != accepted.session_epoch
            || current.document.id != variant_id
            || current.token != accepted.token
            || current.document.revision != accepted.document.revision
        {
            return Err("A newer project action superseded matrix variant navigation.".into());
        }
        if let Some(outcome) = outcome.borrow_mut().take() {
            if outcome != TerminalOutcome::Completed {
                return Err(format!(
                    "The copied project could not select the original board context: {outcome:?}"
                ));
            }
            return exact_variant_snapshot(runtime, &accepted, variant_id, source_scope);
        }
        gloo_timers::future::TimeoutFuture::new(25).await;
    }
    Err("The copied project board context did not settle.".into())
}

fn variant_session_matches(
    expected_session: boardstudio_application::SessionEpoch,
    expected_document: &str,
    actual_session: boardstudio_application::SessionEpoch,
    actual_document: &str,
) -> bool {
    actual_session == expected_session && actual_document == expected_document
}

async fn wait_for_matrix_project(
    runtime: &Rc<Runtime>,
    outcome: OutcomeSlot,
    expected_id: &str,
    expected_session_epoch: boardstudio_application::SessionEpoch,
    alive: &Rc<Cell<bool>>,
) -> Result<boardstudio_application::AcceptedSnapshot, String> {
    for _ in 0..1_200 {
        if !alive.get() {
            return Err("The matrix design variant owner closed.".into());
        }
        if let Some(outcome) = outcome.borrow_mut().take() {
            if outcome != TerminalOutcome::Completed {
                return Err(format!(
                    "The project operation did not complete: {outcome:?}"
                ));
            }
            let model = runtime.model();
            let accepted = model
                .accepted
                .ok_or_else(|| "The project operation was not accepted.".to_owned())?;
            if accepted.document.id != expected_id
                || accepted.session_epoch != expected_session_epoch
            {
                return Err("The project operation was superseded by another open action.".into());
            }
            if model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: accepted.document.revision,
                    })
            {
                return Err(
                    "The project operation completed before its save was confirmed.".into(),
                );
            }
            return Ok(accepted);
        }
        gloo_timers::future::TimeoutFuture::new(25).await;
    }
    Err("The duplicated project did not finish saving.".into())
}

fn is_reversible(document: &ProjectDoc) -> bool {
    document
        .parameters
        .get("reversibleLayout")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn matrix_setup_preset(preset: MatrixPreset) -> crate::matrix_setup_operation::MatrixSetupPreset {
    use crate::matrix_setup_operation::MatrixSetupPreset as Setup;
    match preset {
        MatrixPreset::MxSolder => Setup::MxSolder,
        MatrixPreset::MxHotswap => Setup::MxHotswap,
        MatrixPreset::ChocSolder => Setup::ChocSolder,
        MatrixPreset::ChocHotswap => Setup::ChocHotswap,
        MatrixPreset::MxRgb => Setup::MxRgb,
        MatrixPreset::ChocRgb => Setup::ChocRgb,
        MatrixPreset::MxHotswapRgb => Setup::MxHotswapRgb,
        MatrixPreset::ChocHotswapRgb => Setup::ChocHotswapRgb,
    }
}

/// Mirrors the pinned `matrixWithPreset` recipe while retaining matrix-only fields and each
/// pre-existing cell's enabled state, local pose and non-diode/non-LED assembly membership.
pub(in crate::presentation) fn matrix_with_preset(
    matrix: &Matrix,
    prepared: &Matrix,
    variant: &str,
    orientation: SwitchOrientation,
) -> Matrix {
    let old_cells = matrix
        .cells
        .iter()
        .map(|cell| ((cell.row, cell.column), cell))
        .collect::<std::collections::HashMap<_, _>>();
    let orientation_rotation = match orientation {
        SwitchOrientation::South => 0.0,
        SwitchOrientation::North => 180.0,
    };
    let cells = prepared
        .cells
        .iter()
        .map(|template| {
            let old = old_cells.get(&(template.row, template.column)).copied();
            let old_rotation = old.and_then(|cell| cell.rotation).unwrap_or(0.0);
            let old_orientation_rotation = old
                .and_then(|cell| cell.variant.as_deref())
                .filter(|variant| variant.starts_with("preset/") && variant.ends_with("/north"))
                .map_or(0.0, |_| 180.0);
            let next_rotation = old_rotation - old_orientation_rotation + orientation_rotation;
            let delta = (next_rotation - old_rotation).to_radians();
            let (sin, cos) = delta.sin_cos();
            let mut cell = template.clone();
            cell.variant = Some(variant.to_owned());
            cell.enabled = old.is_none_or(|cell| cell.enabled);
            cell.offset = old.and_then(|cell| cell.offset);
            cell.rotation = Some(next_rotation);
            cell.assemblies_local = Some(true);
            if let Some(old) = old {
                cell.assemblies.extend(
                    old.assemblies
                        .iter()
                        .filter(|assembly| assembly.id != "diode" && assembly.id != "led")
                        .map(|assembly| {
                            let mut assembly = assembly.clone();
                            assembly.offset = Vec2 {
                                x: assembly.offset.x * cos + assembly.offset.y * sin,
                                y: -assembly.offset.x * sin + assembly.offset.y * cos,
                            };
                            assembly.rotation =
                                Some(assembly.rotation.unwrap_or(0.0) - delta.to_degrees());
                            assembly.side.get_or_insert(Side::Front);
                            assembly
                        }),
                );
            }
            cell
        })
        .collect();
    let mut replacement = matrix.clone();
    replacement.definition_id = prepared.definition_id.clone();
    replacement.cells = cells;
    replacement
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn variant_owner_does_not_claim_a_later_open_even_with_the_same_project_id() {
        let original_operation = boardstudio_application::SessionEpoch(12);
        let reopened_variant = boardstudio_application::SessionEpoch(13);
        assert!(variant_session_matches(
            original_operation,
            "variant-id",
            original_operation,
            "variant-id",
        ));
        assert!(!variant_session_matches(
            original_operation,
            "variant-id",
            reopened_variant,
            "variant-id",
        ));
        assert!(!variant_session_matches(
            original_operation,
            "variant-id",
            original_operation,
            "user-opened-project",
        ));
    }

    fn reviung_document() -> ProjectDoc {
        serde_json::from_str(include_str!(
            "../../../../core/tests/fixtures/reviung41-outline-original.json"
        ))
        .expect("checked-in Reviung 41 project fixture should deserialize")
    }

    fn reviung_matrix() -> Matrix {
        let document = reviung_document();
        document
            .matrices
            .into_iter()
            .find(|matrix| matrix.id == "main-right-keys")
            .expect("fixture has the right-hand 3 by 6 matrix")
    }

    fn request_matrix(matrix: &Matrix, field: MatrixEditField, value: MatrixEditValue) -> Matrix {
        let document = reviung_document();
        match build_operation(&document, matrix, &MatrixNameTarget::Matrix, field, value)
            .expect("the selected matrix field should produce a SetMatrix request")
        {
            EditOperation::SetMatrix { matrix, .. } => matrix,
            _ => panic!("matrix fields should use SetMatrix"),
        }
    }

    #[wasm_bindgen_test]
    fn resizing_real_preset_matrix_preserves_cells_and_inherits_only_electrical_members() {
        let mut matrix = reviung_matrix();
        matrix.rows = 2;
        matrix.columns = 2;
        let old_rows = matrix.rows;
        let old_columns = matrix.columns;
        matrix
            .cells
            .retain(|cell| cell.row < old_rows && cell.column < old_columns);
        let original = matrix.cells[0].clone();
        matrix.cells[0].variant = Some("preset/mx-hotswap/north".into());
        matrix.cells[0].assemblies.push(MatrixAssembly {
            id: "led".into(),
            definition_id: "fixture-led".into(),
            offset: boardstudio_core::model::Vec2 { x: 2.0, y: -3.0 },
            rotation: Some(27.0),
            side: Some(boardstudio_core::model::Side::Back),
        });
        matrix.cells[0].assemblies.push(MatrixAssembly {
            id: "local-extra".into(),
            definition_id: "fixture-extra".into(),
            offset: boardstudio_core::model::Vec2 { x: 9.0, y: 8.0 },
            rotation: None,
            side: None,
        });
        let template = matrix.cells[0].clone();

        let resized = request_matrix(
            &matrix,
            MatrixEditField::Columns,
            MatrixEditValue::Columns(3),
        );
        assert_eq!(resized.rows, 2);
        assert_eq!(resized.columns, 3);
        assert_eq!(resized.cells[0].row, original.row);
        assert_eq!(resized.cells[0].offset, original.offset);
        assert_eq!(resized.cells[0].enabled, original.enabled);
        let added = resized
            .cells
            .iter()
            .find(|cell| cell.row == 1 && cell.column == 2)
            .unwrap();
        assert!(added.enabled);
        assert_eq!(
            added.definition_id.as_deref(),
            Some(matrix.definition_id.as_str())
        );
        assert_eq!(added.variant.as_deref(), Some("preset/mx-hotswap/north"));
        assert_eq!(added.rotation, Some(180.0));
        assert_eq!(added.offset, None);
        assert_eq!(added.assemblies_local, template.assemblies_local);
        assert_eq!(
            added.assemblies,
            template
                .assemblies
                .iter()
                .filter(|assembly| assembly.id == "diode" || assembly.id == "led")
                .cloned()
                .collect::<Vec<_>>()
        );
        assert_eq!(added.assemblies.len(), 2);
        assert!(
            !added
                .assemblies
                .iter()
                .any(|assembly| assembly.id == "local-extra")
        );
        assert_eq!(matrix.cells[0].assemblies.len(), template.assemblies.len());
    }

    #[wasm_bindgen_test]
    fn resize_preserves_sparse_holes_and_trims_only_cells_outside_new_bounds() {
        let mut matrix = reviung_matrix();
        matrix.rows = 2;
        matrix.columns = 2;
        let old_rows = matrix.rows;
        let old_columns = matrix.columns;
        matrix
            .cells
            .retain(|cell| cell.row < old_rows && cell.column < old_columns);
        matrix
            .cells
            .retain(|cell| !(cell.row == 1 && cell.column == 1));
        matrix.cells.push(MatrixCell {
            row: 2,
            column: 5,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: Vec::new(),
            assemblies_local: None,
        });
        let before = matrix
            .cells
            .iter()
            .filter(|cell| cell.row < 2 && cell.column < 2)
            .cloned()
            .collect::<Vec<_>>();
        let resized = request_matrix(&matrix, MatrixEditField::Rows, MatrixEditValue::Rows(3));
        let retained = resized
            .cells
            .iter()
            .filter(|cell| cell.row < 2 && cell.column < 2)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(retained, before);
        assert!(
            !resized
                .cells
                .iter()
                .any(|cell| cell.row == 1 && cell.column == 1)
        );
        assert!(
            !resized
                .cells
                .iter()
                .any(|cell| cell.row == 2 && cell.column == 5)
        );
        assert!(
            resized
                .cells
                .iter()
                .any(|cell| cell.row == 2 && cell.column == 1)
        );
    }

    #[wasm_bindgen_test]
    fn custom_resize_leaves_cells_unchanged_and_rejects_oversized_dimensions_before_allocation() {
        let mut matrix = reviung_matrix();
        matrix.cells.iter_mut().for_each(|cell| cell.variant = None);
        let cells = matrix.cells.clone();
        let resized = request_matrix(&matrix, MatrixEditField::Rows, MatrixEditValue::Rows(4));
        assert_eq!(resized.cells, cells);
        assert!(
            matrix_edit(
                &matrix,
                MatrixEditField::Rows,
                MatrixEditValue::Rows(u32::MAX),
            )
            .is_err()
        );
        assert!(matrix_edit(&matrix, MatrixEditField::Rows, MatrixEditValue::Rows(0),).is_err());
    }

    #[wasm_bindgen_test]
    fn matrix_pitch_request_preserves_every_unrelated_accepted_matrix_field() {
        let matrix = reviung_matrix();
        let next = request_matrix(
            &matrix,
            MatrixEditField::PitchX,
            MatrixEditValue::PitchX(20.0),
        );
        assert_eq!(next.pitch.x, 20.0);
        assert_eq!(next.pitch.y, matrix.pitch.y);
        assert_eq!(next.rows, matrix.rows);
        assert_eq!(next.columns, matrix.columns);
        assert_eq!(next.cells, matrix.cells);
        assert_eq!(next.part_ids, matrix.part_ids);
        assert_eq!(next.board_id, matrix.board_id);
    }

    #[wasm_bindgen_test]
    fn owning_layout_name_request_uses_set_layout_without_renaming_the_matrix() {
        let document = reviung_document();
        let matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "main-right-keys")
            .unwrap();
        let layout = document
            .layouts
            .iter()
            .find(|layout| layout.matrix_id == matrix.id && layout.board_id == "main")
            .unwrap();
        let operation = build_operation(
            &document,
            matrix,
            &MatrixNameTarget::Layout {
                id: layout.id.clone(),
            },
            MatrixEditField::Name,
            MatrixEditValue::Name(Some("Renamed layout".into())),
        )
        .unwrap();
        let EditOperation::SetLayout { layout: renamed } = operation else {
            panic!("layout name changes should use SetLayout");
        };
        assert_eq!(renamed.id, layout.id);
        assert_eq!(renamed.matrix_id, matrix.id);
        assert_eq!(renamed.name, "Renamed layout");
        assert_eq!(matrix.name.as_deref(), Some("right keys"));
    }
}
