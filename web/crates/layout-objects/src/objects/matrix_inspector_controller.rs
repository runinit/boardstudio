//! Fresh accepted-source admission and exact operation acknowledgement for matrix fields.
use super::matrix_inspector::{
    MatrixDeleteRequest, MatrixDuplicateRequest, MatrixEditFeedback, MatrixEditField,
    MatrixEditRequest, MatrixEditValue, MatrixInspectorOwner, MatrixInspectorProjection,
    MatrixLayoutRelation, MatrixNameTarget, MatrixPreset, MatrixPresetRequest, MatrixUnlinkRequest,
    SwitchOrientation,
};
use super::{ScopedTreeContext, TreeContext};
use crate::{operation_outcomes::OutcomeSlot, runtime::Runtime};
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope, SnapshotToken,
    TerminalOutcome,
};
use boardstudio_model::model::{
    DiodeDirection, EditOperation, Matrix, PartDefinition, ProjectDoc, Side, Vec2,
};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;

/// A matrix preset prepared from the Parts catalogue: the template matrix and the
/// definitions it needs. The edit applies it to whatever matrix is accepted at execution.
#[derive(Clone)]
struct PreparedPreset {
    matrix: Matrix,
    definitions: Vec<PartDefinition>,
    variant: String,
    orientation: SwitchOrientation,
}

fn matrix_of<'a>(document: &'a ProjectDoc, matrix_id: &str) -> Option<&'a Matrix> {
    document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
}

fn commit(operation: EditOperation, target_ids: Vec<String>) -> Resolution {
    Resolution::submit(target_ids, operation)
}

/// Resolve one field edit against the accepted document at execution time: the matrix (or
/// layout) is read from the accepted document and only the user's value is applied.
fn matrix_field_resolver(
    matrix_id: String,
    name_target: MatrixNameTarget,
    field: MatrixEditField,
    value: MatrixEditValue,
    catalogue_definition: Option<PartDefinition>,
) -> EditResolver {
    EditResolver::new(
        "layout-matrix-inspector",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            let Some(matrix) = matrix_of(document, &matrix_id) else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            if field_value(document, &matrix_id, &name_target, field).as_ref() == Some(&value) {
                return Resolution::Unchanged;
            }
            let mut operation =
                match build_operation(document, matrix, &name_target, field, value.clone()) {
                    Ok(operation) => operation,
                    Err(message) => return Resolution::Retire(message),
                };
            if let (
                MatrixEditField::SwitchDefinition,
                MatrixEditValue::SwitchDefinition(definition_id),
                EditOperation::SetMatrix { definitions, .. },
            ) = (&field, &value, &mut operation)
                && !document
                    .definitions
                    .iter()
                    .any(|definition| definition.id == *definition_id)
            {
                let Some(definition) = catalogue_definition.clone() else {
                    return Resolution::Retire(
                        "The selected switch footprint is not available in the loaded catalogue."
                            .into(),
                    );
                };
                *definitions = Some(vec![definition]);
            }
            let target_id = match &name_target {
                MatrixNameTarget::Layout { id } if field == MatrixEditField::Name => id.clone(),
                _ => matrix_id.clone(),
            };
            commit(operation, vec![target_id])
        },
    )
}

/// Resolve applying a prepared preset to the accepted matrix.
fn matrix_preset_resolver(matrix_id: String, prepared: PreparedPreset) -> EditResolver {
    EditResolver::new(
        "layout-matrix-preset",
        move |accepted: &AcceptedSnapshot| {
            let Some(matrix) = matrix_of(&accepted.document, &matrix_id) else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            if matrix.rows != prepared.matrix.rows || matrix.columns != prepared.matrix.columns {
                return Resolution::Retire(
                    "The matrix size changed before the preset was applied. Apply the preset again."
                        .into(),
                );
            }
            let replacement = matrix_with_preset(
                matrix,
                &prepared.matrix,
                &prepared.variant,
                prepared.orientation,
            );
            if &replacement == matrix {
                return Resolution::Unchanged;
            }
            commit(
                EditOperation::SetMatrix {
                    matrix: replacement,
                    definitions: Some(prepared.definitions.clone()),
                },
                vec![matrix_id.clone()],
            )
        },
    )
}

/// Resolve deleting a matrix that must still exist when the edit runs.
fn matrix_delete_resolver(matrix_id: String) -> EditResolver {
    EditResolver::new(
        "layout-matrix-delete",
        move |accepted: &AcceptedSnapshot| {
            let Some(matrix) = matrix_of(&accepted.document, &matrix_id) else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            let mut target_ids = vec![matrix.id.clone()];
            target_ids.extend(matrix.part_ids.iter().cloned());
            commit(
                EditOperation::RemoveMatrix {
                    id: matrix.id.clone(),
                },
                target_ids,
            )
        },
    )
}

/// Resolve unlinking mirrored halves: the layout must still exist and still be linked.
fn unlink_layout_resolver(board_id: String, layout_id: String) -> EditResolver {
    EditResolver::new(
        "layout-matrix-unlink",
        move |accepted: &AcceptedSnapshot| {
            let Some(mut layout) = accepted
                .document
                .layouts
                .iter()
                .find(|layout| layout.id == layout_id && layout.board_id == board_id)
                .cloned()
            else {
                return Resolution::Retire("The selected layout no longer exists.".into());
            };
            if layout.mirror_link.is_none() {
                return Resolution::Unchanged;
            }
            layout.mirror_link = None;
            commit(EditOperation::SetLayout { layout }, vec![layout_id.clone()])
        },
    )
}

#[derive(Clone)]
struct VariantReportOwner {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
}

enum VariantEditWait {
    Landed,
    Retired,
}

enum VariantSetupResult {
    Completed,
    Retired,
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
pub struct MatrixInspectorMount {
    pub projection: Option<MatrixInspectorProjection>,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    /// A one-shot action (preset, delete, unlink, duplicate) is pending or preparing.
    /// Field edits never read this: they queue freely.
    pub busy: bool,
    pub inspector_mounted: Signal<bool>,
    pub add_row_pending: bool,
    pub add_column_pending: bool,
    pub feedback: Vec<MatrixEditFeedback>,
    pub on_edit: EventHandler<MatrixEditRequest>,
    pub on_add_row: EventHandler<()>,
    pub on_add_column: EventHandler<()>,
    pub on_apply_preset: EventHandler<MatrixPresetRequest>,
    pub on_delete: EventHandler<MatrixDeleteRequest>,
    pub on_unlink: EventHandler<MatrixUnlinkRequest>,
    pub on_duplicate: EventHandler<MatrixDuplicateRequest>,
}

/// Row and column growth share one submission path: `field` selects which
/// dimension advances by one, guarded by its own pending edit.
#[allow(clippy::too_many_arguments)]
fn submit_dimension_bump(
    runtime: &Runtime,
    selected_context: Signal<Option<ScopedTreeContext>>,
    context_generation: &RefCell<ContextGeneration>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    editor_instance_id: u64,
    mut request_sequence: Signal<u64>,
    on_edit: EventHandler<MatrixEditRequest>,
    field: MatrixEditField,
) {
    let selected = selected_context.read().clone();
    let model = runtime.model();
    let scope = runtime.scope();
    let Some((projection, editable)) = project_current_for(
        runtime,
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
    let baseline = match field {
        MatrixEditField::Rows => MatrixEditValue::Rows(projection.rows),
        MatrixEditField::Columns => MatrixEditValue::Columns(projection.columns),
        _ => return,
    };
    let grown = match baseline {
        MatrixEditValue::Rows(rows) => rows.checked_add(1).map(MatrixEditValue::Rows),
        MatrixEditValue::Columns(columns) => columns.checked_add(1).map(MatrixEditValue::Columns),
        _ => None,
    };
    let Some(value) = grown else {
        return;
    };
    if !editable {
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
        field,
        baseline,
        value,
    });
}

/// Must be called unconditionally at the shared presentation lifetime. It owns no document
/// state: every projection and callback resolves the current accepted snapshot again.
pub fn use_matrix_inspector(
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
    let _ = version();
    let inspector_mounted = use_signal(|| false);
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
    let (fresh_projection, editable) = project_current(
        &runtime,
        selected.as_ref(),
        editor_instance_id,
        generation,
        current_scope_generation,
        current_workspace,
    );
    let fresh_projection = fresh_projection.map(|mut projection| {
        if let Some(document) = runtime.model().accepted.map(|snapshot| snapshot.document) {
            projection.switch_choices =
                switch_choices(&document, &switch_catalog(), &projection.definition_id);
        }
        projection
    });
    let projection = fresh_projection;
    let on_edit = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        let mut last_request_id = last_request_id;
        let mut feedback = feedback;
        move |request: MatrixEditRequest| {
            let current_generation = context_generation.borrow().value;
            let current_workspace = workspace();
            let current_scope_generation = scope_generation();
            if request.owner.editor_instance_id != editor_instance_id
                || request.owner.context_generation != current_generation
                || request.owner.scope_generation != current_scope_generation
                || current_workspace != "Layout"
                || !inspector_mounted()
                || request.request_id <= last_request_id()
            {
                return;
            }
            last_request_id.set(request.request_id);
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
                    "The matrix is not ready to edit. Wait for the current operation to finish, then retry.".into(),
                );
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let Some(matrix) = snapshot
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == request.owner.matrix_id)
            else {
                return;
            };
            // Validate the typed value now so a bad entry is explained inline; the resolver
            // repeats the build against whatever is accepted when the edit runs.
            if let Err(message) = build_operation(
                &snapshot.document,
                matrix,
                &request.owner.name_target,
                request.field,
                request.value.clone(),
            ) {
                publish_feedback(&mut feedback, &request, message);
                return;
            }
            let catalogue_definition = match &request.value {
                MatrixEditValue::SwitchDefinition(definition_id)
                    if request.field == MatrixEditField::SwitchDefinition =>
                {
                    let catalogue = switch_catalog()
                        .into_iter()
                        .find(|definition| definition.id == *definition_id);
                    if catalogue.is_none()
                        && !snapshot
                            .document
                            .definitions
                            .iter()
                            .any(|definition| definition.id == *definition_id)
                    {
                        publish_feedback(
                            &mut feedback,
                            &request,
                            "The selected switch footprint is not available in the loaded catalogue.".into(),
                        );
                        return;
                    }
                    catalogue
                }
                _ => None,
            };
            let result = runtime.commit(
                "layout-matrix-inspector",
                matrix_field_resolver(
                    request.owner.matrix_id.clone(),
                    request.owner.name_target.clone(),
                    request.field,
                    request.value.clone(),
                    catalogue_definition,
                ),
            );
            if let Err(message) = result {
                publish_feedback(&mut feedback, &request, message);
            }
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
                || !inspector_mounted()
                || request.request_id <= last_request_id()
            {
                return;
            }
            last_request_id.set(request.request_id);
            if preparing_preset() || duplicating() {
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
            if !editable {
                publish_action_feedback(
                    &mut feedback,
                    &request.owner,
                    request.request_id,
                    MatrixEditField::ApplyPreset,
                    "The matrix is not ready to edit. Wait for the current operation to finish, then retry.".into(),
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
            let reversible = is_reversible(&document);
            preparing_preset.set(true);
            let runtime = runtime.clone();
            let mut preparing = preparing_preset;
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
                let request_owner_is_current = runtime.scope().as_ref()
                    == Some(&request.owner.scope)
                    && scope_generation() == request.owner.scope_generation
                    && context_generation.borrow().value == request.owner.context_generation
                    && workspace() == "Layout"
                    && inspector_mounted();
                if !request_owner_is_current {
                    clear_action_feedback(&mut feedback, &request.owner, request.request_id);
                    return;
                }
                let prepared = match result {
                    Ok(prepared) => prepared,
                    Err(message) => {
                        publish_action_feedback(
                            &mut feedback,
                            &request.owner,
                            request.request_id,
                            MatrixEditField::ApplyPreset,
                            message,
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
                    clear_action_feedback(&mut feedback, &request.owner, request.request_id);
                    return;
                };
                if current.owner != request.owner {
                    clear_action_feedback(&mut feedback, &request.owner, request.request_id);
                    return;
                }
                if !editable {
                    publish_action_feedback(
                        &mut feedback,
                        &request.owner,
                        request.request_id,
                        MatrixEditField::ApplyPreset,
                        "The matrix is not ready to edit. Wait for the current operation to finish, then retry.".into(),
                    );
                    return;
                }
                if !alive.get() {
                    return;
                }
                match runtime.commit(
                    "layout-matrix-preset",
                    matrix_preset_resolver(request.owner.matrix_id.clone(), prepared),
                ) {
                    Ok(_) => {
                        clear_action_feedback(&mut feedback, &request.owner, request.request_id)
                    }
                    Err(message) => publish_action_feedback(
                        &mut feedback,
                        &request.owner,
                        request.request_id,
                        MatrixEditField::ApplyPreset,
                        message,
                    ),
                }
            });
        }
    });

    let on_delete = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        move |request: MatrixDeleteRequest| {
            if request.owner.editor_instance_id != editor_instance_id
                || request.owner.context_generation != context_generation.borrow().value
                || request.owner.scope_generation != scope_generation()
                || workspace() != "Layout"
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
            if projection.owner != request.owner || !editable {
                runtime.report(
                    "The selected matrix changed. Reopen the inspector before deleting it.",
                );
                return;
            }
            let result = runtime.commit(
                "layout-matrix-delete",
                matrix_delete_resolver(request.owner.matrix_id),
            );
            match result {
                Ok(_) => {
                    let mut selected_context = selected_context;
                    selected_context.set(None);
                    runtime.submit(Event::SelectParts {
                        operation_id: runtime.operation(),
                        part_ids: Vec::new(),
                        range_part_ids: Vec::new(),
                        mode: boardstudio_application::SelectionMode::Replace,
                    });
                }
                Err(message) => runtime.report(format!("Could not delete matrix: {message}")),
            }
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
            if projection.owner != request.owner || !editable {
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
                if matches!(&result, Ok(VariantSetupResult::Retired)) {
                    return;
                }
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

    // Row and column growth share one submission path: the field selects which
    // dimension advances by one, guarded by its own pending edit.
    let on_add_row = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        move |()| {
            submit_dimension_bump(
                &runtime,
                selected_context,
                &context_generation,
                scope_generation,
                workspace,
                editor_instance_id,
                request_sequence,
                on_edit,
                MatrixEditField::Rows,
            );
        }
    });

    let on_add_column = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        move |()| {
            submit_dimension_bump(
                &runtime,
                selected_context,
                &context_generation,
                scope_generation,
                workspace,
                editor_instance_id,
                request_sequence,
                on_edit,
                MatrixEditField::Columns,
            );
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
            if projection.owner != request.owner
                || !editable
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
            if let Err(message) = runtime.commit(
                "layout-matrix-unlink",
                unlink_layout_resolver(request.owner.scope.board_id, request.layout_id),
            ) {
                runtime.report(format!("Could not unlink the halves: {message}"));
            }
        }
    });

    MatrixInspectorMount {
        projection,
        request_sequence,
        editable,
        busy: preparing_preset() || duplicating(),
        inspector_mounted,
        add_row_pending: false,
        add_column_pending: false,
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

mod edits;
mod projection;
mod variant;

use edits::{
    build_operation, clear_action_feedback, duplicate_design_variant, field_value,
    prepare_preset_matrix, publish_action_feedback, publish_feedback,
};
use projection::{MatrixProjectionContext, project_current, project_current_for};
use variant::is_reversible;

pub use projection::switch_choices;
pub use variant::matrix_with_preset;
