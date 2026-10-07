//! Accepted-snapshot projection and exact operation lifecycle for transform fields.
use super::matrix_transform_inspector::{
    MatrixTransformFeedback, MatrixTransformInspectorOwner, MatrixTransformProjection,
    MatrixTransformRequest, MatrixTransformState,
};
use super::{ScopedTreeContext, TreeContext};
use crate::matrix_transform_operation::{
    MatrixTransformField, MatrixTransformFields, MatrixTransformValue, build_operation,
    catalogue_retry_due, component_edit_admitted_by_catalogue,
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_core::model::{
    EditOperation, Matrix, MatrixSplayAffect, PartDefinition, ProjectDoc, Vec2,
};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
struct TransformSubmission {
    request: MatrixTransformRequest,
    ticket: EditTicket,
}

/// Resolve one transform field edit against the accepted document at execution time. The
/// matrix and the row, column or key the field belongs to are read from the accepted
/// document and only the user's committed value is applied, so queued edits compose and the
/// latest committed value wins. `catalogue_definitions` are the catalogue entries the value
/// may need; only those the accepted document lacks are added.
pub fn transform_edit_resolver(
    matrix_id: String,
    board_id: String,
    context: TreeContext,
    field: MatrixTransformField,
    value: MatrixTransformValue,
    splay_affect: MatrixSplayAffect,
    catalogue_definitions: Vec<PartDefinition>,
) -> EditResolver {
    EditResolver::new(
        "layout-matrix-transform",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            let Some(matrix) = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
            else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            let Some(fields) = transform_fields(accepted, &board_id, &context, matrix) else {
                return Resolution::Retire(
                    "The selected row, column or key no longer exists.".into(),
                );
            };
            if field_value(&fields, field).as_ref() == Some(&value) {
                return Resolution::Unchanged;
            }
            let operation = match build_operation(
                matrix,
                &fields,
                field,
                value.clone(),
                splay_affect.clone(),
            ) {
                Ok(operation) => operation,
                Err(message) => return Resolution::Retire(message),
            };
            let missing = |id: &String| {
                !document
                    .definitions
                    .iter()
                    .any(|definition| &definition.id == id)
            };
            let operation = match (field, operation, &value) {
                (
                    MatrixTransformField::KeyAssembly,
                    EditOperation::SetMatrix { matrix, .. },
                    MatrixTransformValue::Text(id),
                ) => EditOperation::SetMatrix {
                    matrix,
                    definitions: missing(id)
                        .then(|| {
                            catalogue_definitions
                                .iter()
                                .find(|definition| &definition.id == id)
                                .cloned()
                        })
                        .flatten()
                        .map(|definition| vec![definition]),
                },
                (
                    MatrixTransformField::KeyAttached,
                    EditOperation::SetMatrix { matrix, .. },
                    MatrixTransformValue::Attached(list),
                ) => {
                    let definitions = list
                        .iter()
                        .filter(|(_, id)| missing(id))
                        .filter_map(|(_, id)| {
                            catalogue_definitions
                                .iter()
                                .find(|definition| &definition.id == id)
                                .cloned()
                        })
                        .collect::<Vec<_>>();
                    EditOperation::SetMatrix {
                        matrix,
                        definitions: (!definitions.is_empty()).then_some(definitions),
                    }
                }
                (_, operation, _) => operation,
            };
            Resolution::submit(vec![matrix.id.clone()], operation)
        },
    )
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

#[derive(Clone, PartialEq)]
pub struct MatrixTransformInspectorMount {
    pub projection: Option<MatrixTransformProjection>,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub feedback: Vec<MatrixTransformFeedback>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
    pub splay_affect: Signal<MatrixSplayAffect>,
}

impl MatrixTransformInspectorMount {
    pub fn pick_splay_origin(&self, point: Vec2) -> bool {
        let Some(projection) = self.projection.as_ref() else {
            return false;
        };
        let MatrixTransformFields::Column { splay_origin, .. } = &projection.fields else {
            return false;
        };
        if !self.editable || !point.x.is_finite() || !point.y.is_finite() {
            return false;
        }
        let Some(request_id) = (self.request_sequence)().checked_add(1) else {
            return false;
        };
        let mut request_sequence = self.request_sequence;
        request_sequence.set(request_id);
        self.on_edit.call(MatrixTransformRequest {
            owner: projection.owner.clone(),
            request_id,
            snapshot_token: projection.snapshot_token,
            revision: projection.revision,
            field: MatrixTransformField::SplayOriginPoint,
            baseline: MatrixTransformValue::Point(*splay_origin),
            value: MatrixTransformValue::Point(point),
            splay_affect: (self.splay_affect)(),
        });
        true
    }
}

/// Called once at the Editor lifetime. Every request and projection resolves the live accepted
/// matrix again; no document copy survives as a writable store.
pub fn use_workspace_matrix_transform(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    splay_affect: Signal<MatrixSplayAffect>,
    owner_workspace: &'static str,
) -> MatrixTransformInspectorMount {
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let context_generation = use_hook(|| Rc::new(RefCell::new(ContextGeneration::default())));
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
                .expect("matrix transform context generation exhausted");
            tracker.identity = observed_identity.clone();
            tracker.initialized = true;
        }
        tracker.value
    };

    let request_sequence = use_signal(|| 0u64);
    let last_request_id = use_signal(|| 0u64);
    let pending = use_signal(Vec::<TransformSubmission>::new);
    let feedback = use_signal(Vec::<MatrixTransformFeedback>::new);
    let alive = use_hook(|| Rc::new(std::cell::Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    let switch_catalog = use_signal(Vec::<boardstudio_core::model::PartDefinition>::new);
    // The bundled catalogue is built per document reversibility (as in the Parts browser), so
    // it is requested once per accepted reversibility and tagged with the build it holds.
    let mut catalog_requested = use_signal(|| None::<bool>);
    let mut catalog_loading = use_signal(|| false);
    let catalog_built_for = use_signal(|| None::<bool>);
    use_effect({
        let alive = alive.clone();
        let runtime = runtime.clone();
        move || {
            let _ = version();
            let Some(reversible) = runtime
                .model()
                .accepted
                .map(|snapshot| crate::presentation::parts::reversible_layout(&snapshot.document))
            else {
                return;
            };
            if catalog_requested() == Some(reversible) {
                return;
            }
            catalog_requested.set(Some(reversible));
            catalog_loading.set(true);
            let mut switch_catalog = switch_catalog;
            let mut catalog_built_for = catalog_built_for;
            let mut catalog_loading = catalog_loading;
            let alive = alive.clone();
            spawn_local(async move {
                let result =
                    crate::presentation::parts::load_all_catalogue_definitions(reversible).await;
                if alive.get() && catalog_requested() == Some(reversible) {
                    catalog_loading.set(false);
                    if let Ok(definitions) = result {
                        switch_catalog.set(definitions);
                        catalog_built_for.set(Some(reversible));
                    }
                }
            });
        }
    });
    let (projection, editable) = project_current(
        &runtime,
        selected.as_ref(),
        editor_instance_id,
        generation,
        current_scope_generation,
        current_workspace,
        owner_workspace,
    );
    let projection = projection.map(|mut projection| {
        if let MatrixTransformFields::Key {
            definition_id,
            choices,
            component_choices,
            ..
        } = &mut projection.fields
            && let Some(document) = runtime.model().accepted.map(|snapshot| snapshot.document)
        {
            *choices = super::matrix_inspector_controller::switch_choices(
                &document,
                &switch_catalog(),
                definition_id,
            );
            *component_choices = document
                .definitions
                .iter()
                .chain(switch_catalog().iter())
                .fold(Vec::new(), |mut all, definition| {
                    if !all
                        .iter()
                        .any(|existing: &boardstudio_core::model::PartDefinition| {
                            existing.id == definition.id
                        })
                    {
                        all.push(definition.clone());
                    }
                    all
                });
        }
        projection
    });
    let identity = observed_identity.clone();

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation(), &identity),
        {
            let runtime = runtime.clone();
            let mut pending = pending;
            let mut feedback = feedback;
            move |(_, _, scope_generation, _)| {
                settle_pending(&runtime, scope_generation, &mut pending, &mut feedback);
            }
        },
    ));

    let on_edit = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        let mut last_request_id = last_request_id;
        let mut pending = pending;
        let mut feedback = feedback;
        let mut catalog_requested = catalog_requested;
        move |request: MatrixTransformRequest| {
            let current_generation = context_generation.borrow().value;
            let current_workspace = workspace();
            let current_scope_generation = scope_generation();
            if request.owner.editor_instance_id != editor_instance_id
                || request.owner.context_generation != current_generation
                || request.owner.scope_generation != current_scope_generation
                || !workspace_route_matches(current_workspace, owner_workspace)
                || request.owner.workspace != owner_workspace
                || request.request_id <= last_request_id()
            {
                return;
            }
            last_request_id.set(request.request_id);
            let Some(selected) = selected_context.read().clone() else {
                return;
            };
            let model = runtime.model();
            let scope = runtime.scope();
            let Some((current, editable)) = project_current_for(
                &runtime,
                &model,
                scope.as_ref(),
                Some(&selected),
                ProjectionContext {
                    editor_instance_id,
                    context_generation: current_generation,
                    scope_generation: current_scope_generation,
                    workspace: current_workspace,
                },
                owner_workspace,
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
                    MatrixTransformState::Failed,
                    Some("The matrix transform is not ready to edit. Wait for the current operation to finish, then retry.".into()),
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
            let reversible = crate::presentation::parts::reversible_layout(&snapshot.document);
            let document_definition_ids = snapshot
                .document
                .definitions
                .iter()
                .map(|definition| definition.id.clone())
                .collect::<Vec<_>>();
            let retry_catalogue = matches!(
                request.field,
                MatrixTransformField::KeyAssembly | MatrixTransformField::KeyAttached
            ) && catalogue_retry_due(
                catalog_requested(),
                catalog_loading(),
                catalog_built_for(),
                reversible,
            );
            if retry_catalogue {
                // A completed failure leaves the requested construction tagged so ordinary
                // renders do not spin. A fresh key action clears it and retries this construction.
                catalog_requested.set(None);
            }
            if !component_edit_admitted_by_catalogue(
                catalog_built_for(),
                reversible,
                request.field,
                &request.baseline,
                &request.value,
                &document_definition_ids,
            ) {
                let message = if catalog_loading() {
                    "The component catalogue is still loading. Try this change again when it finishes."
                } else if retry_catalogue {
                    "The component catalogue failed to load. Its retry has started; try this change again when it finishes."
                } else {
                    "The component catalogue is not ready yet. Try this change again when it finishes loading."
                };
                publish_feedback(
                    &mut feedback,
                    &request,
                    MatrixTransformState::Failed,
                    Some(message.into()),
                );
                return;
            }
            // Validate the typed value now so a bad entry is explained inline; the resolver
            // repeats the build against whatever is accepted when the edit runs.
            if let Err(message) = build_operation(
                matrix,
                &current.fields,
                request.field,
                request.value.clone(),
                request.splay_affect.clone(),
            ) {
                publish_feedback(
                    &mut feedback,
                    &request,
                    MatrixTransformState::Failed,
                    Some(message),
                );
                return;
            }
            let catalogue_definitions = match (&request.field, &request.value) {
                (MatrixTransformField::KeyAssembly, MatrixTransformValue::Text(id)) => {
                    switch_catalog()
                        .into_iter()
                        .filter(|definition| &definition.id == id)
                        .collect()
                }
                (MatrixTransformField::KeyAttached, MatrixTransformValue::Attached(list)) => {
                    switch_catalog()
                        .into_iter()
                        .filter(|definition| list.iter().any(|(_, id)| id == &definition.id))
                        .collect()
                }
                _ => Vec::new(),
            };
            let ticket = EditTicket::begin(
                &runtime,
                "layout-matrix-transform",
                Some("transform".into()),
                transform_edit_resolver(
                    request.owner.matrix_id.clone(),
                    request.owner.scope.board_id.clone(),
                    request.owner.context.clone(),
                    request.field,
                    request.value.clone(),
                    request.splay_affect.clone(),
                    catalogue_definitions,
                ),
            );
            pending.write().push(TransformSubmission {
                request: request.clone(),
                ticket,
            });
            publish_feedback(&mut feedback, &request, MatrixTransformState::Pending, None);
        }
    });

    MatrixTransformInspectorMount {
        projection,
        request_sequence,
        editable,
        feedback: feedback.read().clone(),
        on_edit,
        splay_affect,
    }
}

fn project_current(
    runtime: &Runtime,
    selected: Option<&ScopedTreeContext>,
    editor_instance_id: u64,
    context_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
    owner_workspace: &'static str,
) -> (Option<MatrixTransformProjection>, bool) {
    if !workspace_route_matches(workspace, owner_workspace) {
        return (None, false);
    }
    let model = runtime.model();
    let scope = runtime.scope();
    project_current_for(
        runtime,
        &model,
        scope.as_ref(),
        selected,
        ProjectionContext {
            editor_instance_id,
            context_generation,
            scope_generation,
            workspace,
        },
        owner_workspace,
    )
    .map_or((None, false), |(projection, editable)| {
        (Some(projection), editable)
    })
}

struct ProjectionContext {
    editor_instance_id: u64,
    context_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
}

fn project_current_for(
    runtime: &Runtime,
    model: &boardstudio_application::ReadModel,
    live_scope: Option<&Scope>,
    selected: Option<&ScopedTreeContext>,
    context: ProjectionContext,
    owner_workspace: &'static str,
) -> Option<(MatrixTransformProjection, bool)> {
    if !workspace_route_matches(context.workspace, owner_workspace) {
        return None;
    }
    let scope = live_scope?;
    let selected = selected?;
    if &selected.scope != scope || runtime.scope().as_ref() != Some(scope) {
        return None;
    }
    let matrix_id = match &selected.context {
        TreeContext::Matrix { matrix_id }
        | TreeContext::Row { matrix_id, .. }
        | TreeContext::Column { matrix_id, .. }
        | TreeContext::Key { matrix_id, .. } => matrix_id,
        _ => return None,
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
    let fields = transform_fields(snapshot, &scope.board_id, &selected.context, matrix)?;
    // Transform edits queue behind each other: the inspector stays editable while an earlier
    // edit is applying or saving.
    let editable = matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) && matches!(
        model.durability,
        Durability::Saved { .. } | Durability::Saving { .. }
    ) && model.display_preview.is_none()
        && model.gesture.is_none();
    Some((
        MatrixTransformProjection {
            owner: MatrixTransformInspectorOwner {
                editor_instance_id: context.editor_instance_id,
                workspace: context.workspace,
                context_generation: context.context_generation,
                scope_generation: context.scope_generation,
                scope: scope.clone(),
                matrix_id: matrix.id.clone(),
                context: selected.context.clone(),
            },
            snapshot_token: snapshot.token,
            revision: snapshot.document.revision,
            label: super::context_label(model, &selected.context)
                .unwrap_or_else(|| format!("Matrix {}", matrix.id)),
            fields,
        },
        editable,
    ))
}

/// The field projection of the selected row, column or key from an accepted snapshot. The
/// Inspector renders it and the edit resolver reads it again at execution time.
fn transform_fields(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    context: &TreeContext,
    matrix: &Matrix,
) -> Option<MatrixTransformFields> {
    Some(match context {
        TreeContext::Matrix { .. } => MatrixTransformFields::Matrix {
            origin: matrix.origin,
            rotation: matrix.rotation.unwrap_or(0.0),
            mirror: matrix.mirror,
            mirror_y_locked: matrix_has_linked_layout(&snapshot.document, board_id, &matrix.id),
        },
        TreeContext::Row { row, .. } => MatrixTransformFields::Row {
            row: *row,
            offset: matrix
                .row_offsets
                .get(*row as usize)
                .copied()
                .unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
        },
        TreeContext::Column { column, .. } => {
            let scene = snapshot
                .scene
                .matrix_scenes
                .iter()
                .find(|scene| scene.matrix_id == matrix.id)?;
            let basis = scene.columns.iter().find(|basis| basis.column == *column)?;
            MatrixTransformFields::Column {
                column: *column,
                offset: matrix
                    .column_offsets
                    .get(*column as usize)
                    .copied()
                    .unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
                stagger: matrix
                    .column_staggers
                    .get(*column as usize)
                    .copied()
                    .unwrap_or(0.0),
                splay_angle: basis.splay_angle,
                splay_origin: basis.splay_origin,
                custom_origin: basis.custom_origin,
            }
        }
        TreeContext::Key { row, column, .. } => {
            let cell = matrix
                .cells
                .iter()
                .find(|cell| cell.row == *row && cell.column == *column);
            MatrixTransformFields::Key {
                row: *row,
                column: *column,
                enabled: cell.is_none_or(|cell| cell.enabled),
                definition_id: cell
                    .and_then(|cell| cell.definition_id.clone())
                    .unwrap_or_else(|| matrix.definition_id.clone()),
                choices: Vec::new(),
                assemblies: cell
                    .map(|cell| {
                        cell.assemblies
                            .iter()
                            .map(|assembly| (assembly.id.clone(), assembly.definition_id.clone()))
                            .collect()
                    })
                    .unwrap_or_default(),
                component_choices: Vec::new(),
                mirror_target: snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|layout| layout.matrix_id == matrix.id && layout.mirror_link.is_some()),
                assemblies_local: cell.and_then(|cell| cell.assemblies_local).unwrap_or(false),
                offset: cell
                    .and_then(|cell| cell.offset)
                    .unwrap_or(Vec2 { x: 0.0, y: 0.0 }),
                rotation: cell.and_then(|cell| cell.rotation).unwrap_or(0.0),
            }
        }
        _ => return None,
    })
}

fn workspace_route_matches(current: &str, owner: &str) -> bool {
    matches!(owner, "Layout" | "PCB") && current == owner
}

#[cfg(all(test, target_arch = "wasm32"))]
mod workspace_route_tests {
    use super::workspace_route_matches;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn matrix_transform_owner_cannot_cross_workspace_routes() {
        assert!(workspace_route_matches("Layout", "Layout"));
        assert!(workspace_route_matches("PCB", "PCB"));
        assert!(!workspace_route_matches("PCB", "Layout"));
        assert!(!workspace_route_matches("Layout", "PCB"));
        assert!(!workspace_route_matches("Case", "Case"));
    }
}

fn matrix_has_linked_layout(document: &ProjectDoc, board_id: &str, matrix_id: &str) -> bool {
    document
        .layouts
        .iter()
        .find(|layout| layout.board_id == board_id && layout.matrix_id == matrix_id)
        .is_some_and(|layout| {
            layout.mirror_link.is_some()
                || document.layouts.iter().any(|other| {
                    other
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == layout.id)
                })
        })
}

fn field_value(
    fields: &MatrixTransformFields,
    field: MatrixTransformField,
) -> Option<MatrixTransformValue> {
    use MatrixTransformField as Field;
    use MatrixTransformValue as Value;
    match (fields, field) {
        (MatrixTransformFields::Matrix { origin, .. }, Field::OriginX) => {
            Some(Value::Number(origin.x))
        }
        (MatrixTransformFields::Matrix { origin, .. }, Field::OriginY) => {
            Some(Value::Number(origin.y))
        }
        (MatrixTransformFields::Matrix { rotation, .. }, Field::MatrixRotation) => {
            Some(Value::Number(*rotation))
        }
        (MatrixTransformFields::Matrix { mirror, .. }, Field::MatrixMirror) => {
            Some(Value::Mirror(*mirror))
        }
        (MatrixTransformFields::Row { offset, .. }, Field::RowOffsetX) => {
            Some(Value::Number(offset.x))
        }
        (MatrixTransformFields::Row { offset, .. }, Field::RowOffsetY) => {
            Some(Value::Number(offset.y))
        }
        (MatrixTransformFields::Row { offset, .. }, Field::RowOffsetReset) => {
            Some(Value::Offset(*offset))
        }
        (MatrixTransformFields::Column { offset, .. }, Field::ColumnOffsetX) => {
            Some(Value::Number(offset.x))
        }
        (MatrixTransformFields::Column { offset, .. }, Field::ColumnOffsetY) => {
            Some(Value::Number(offset.y))
        }
        (MatrixTransformFields::Column { offset, .. }, Field::ColumnOffsetReset) => {
            Some(Value::Offset(*offset))
        }
        (MatrixTransformFields::Column { stagger, .. }, Field::ColumnStagger) => {
            Some(Value::Number(*stagger))
        }
        (MatrixTransformFields::Column { splay_angle, .. }, Field::ColumnSplay) => {
            Some(Value::Number(*splay_angle))
        }
        (MatrixTransformFields::Column { custom_origin, .. }, Field::SplayOriginMode) => {
            Some(Value::OriginMode(*custom_origin))
        }
        (MatrixTransformFields::Column { splay_origin, .. }, Field::SplayOriginX) => {
            Some(Value::Number(splay_origin.x))
        }
        (MatrixTransformFields::Column { splay_origin, .. }, Field::SplayOriginY) => {
            Some(Value::Number(splay_origin.y))
        }
        (MatrixTransformFields::Column { splay_origin, .. }, Field::SplayOriginPoint) => {
            Some(Value::Point(*splay_origin))
        }
        (MatrixTransformFields::Key { offset, .. }, Field::KeyOffsetX) => {
            Some(Value::Number(offset.x))
        }
        (MatrixTransformFields::Key { offset, .. }, Field::KeyOffsetY) => {
            Some(Value::Number(offset.y))
        }
        (MatrixTransformFields::Key { rotation, .. }, Field::KeyRotation) => {
            Some(Value::Number(*rotation))
        }
        (MatrixTransformFields::Key { enabled, .. }, Field::KeyEnabled) => {
            Some(Value::Bool(*enabled))
        }
        (MatrixTransformFields::Key { assemblies, .. }, Field::KeyAttached) => {
            Some(Value::Attached(assemblies.clone()))
        }
        (MatrixTransformFields::Key { definition_id, .. }, Field::KeyAssembly) => {
            Some(Value::Text(definition_id.clone()))
        }
        (
            MatrixTransformFields::Key {
                assemblies_local, ..
            },
            Field::KeyAssembliesLocal,
        ) => Some(Value::Bool(*assemblies_local)),
        (
            MatrixTransformFields::Key {
                offset, rotation, ..
            },
            Field::KeyTransformReset,
        ) => Some(Value::CellTransform {
            offset: *offset,
            rotation: *rotation,
        }),
        _ => None,
    }
}

fn settle_pending(
    runtime: &Runtime,
    scope_generation: u64,
    pending: &mut Signal<Vec<TransformSubmission>>,
    feedback: &mut Signal<Vec<MatrixTransformFeedback>>,
) {
    let waiting = pending.read().clone();
    if waiting.is_empty() {
        return;
    }
    let scope = runtime.scope();
    let mut remaining = Vec::with_capacity(waiting.len());
    let mut changed = false;
    for edit in waiting {
        let live = scope.as_ref() == Some(&edit.request.owner.scope)
            && scope_generation == edit.request.owner.scope_generation;
        match edit.ticket.settlement(live) {
            Settlement::Pending => remaining.push(edit),
            Settlement::Landed { .. } => {
                changed = true;
                publish_feedback(feedback, &edit.request, MatrixTransformState::Saved, None);
            }
            Settlement::Failed { message } => {
                changed = true;
                publish_feedback(
                    feedback,
                    &edit.request,
                    MatrixTransformState::Failed,
                    Some(message),
                );
            }
            Settlement::Retired => {
                changed = true;
                if live {
                    publish_feedback(
                        feedback,
                        &edit.request,
                        MatrixTransformState::Failed,
                        Some("The transform did not complete in the active session. Review the current value and retry.".into()),
                    );
                } else {
                    feedback.set(Vec::new());
                }
            }
        }
    }
    if changed {
        pending.set(remaining);
    }
}

fn publish_feedback(
    feedback: &mut Signal<Vec<MatrixTransformFeedback>>,
    request: &MatrixTransformRequest,
    state: MatrixTransformState,
    message: Option<String>,
) {
    let next = MatrixTransformFeedback {
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
        let Some(index) = entries
            .iter()
            .position(|entry| entry.state != MatrixTransformState::Pending)
        else {
            break;
        };
        entries.remove(index);
    }
    feedback.set(entries);
}

#[cfg(all(test, target_arch = "wasm32"))]
mod queued_edits {
    use super::*;
    use crate::presentation::objects::matrix_edit_test_support as fixture;
    use crate::runtime::project_name_test_support as support;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn origin_ticket(runtime: &Rc<Runtime>, field: MatrixTransformField, value: f64) -> EditTicket {
        EditTicket::begin(
            runtime,
            "layout-matrix-transform",
            Some("transform".into()),
            transform_edit_resolver(
                fixture::MATRIX_ID.into(),
                fixture::BOARD_ID.into(),
                TreeContext::Matrix {
                    matrix_id: fixture::MATRIX_ID.into(),
                },
                field,
                MatrixTransformValue::Number(value),
                MatrixSplayAffect::Column,
                Vec::new(),
            ),
        )
    }

    #[wasm_bindgen_test]
    async fn two_transform_fields_committed_back_to_back_both_survive() {
        let runtime = fixture::open_matrix_runtime().await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let x = origin_ticket(&runtime, MatrixTransformField::OriginX, 5.0);
        support::drive_pending(&runtime);
        entered.await.expect("the X edit reached Core");
        let y = origin_ticket(&runtime, MatrixTransformField::OriginY, 7.0);
        support::drive_pending(&runtime);
        release.send(()).expect("release the held X reply");
        fixture::settle_ticket(&runtime, &y).await;

        assert!(matches!(x.settlement(true), Settlement::Landed { .. }));
        assert!(matches!(y.settlement(true), Settlement::Landed { .. }));
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.origin,
            Vec2 { x: 5.0, y: 7.0 },
            "the Y edit resolved against the document the X edit produced"
        );
    }

    #[wasm_bindgen_test]
    async fn a_committed_transform_value_wins_over_a_changed_baseline() {
        let runtime = fixture::open_matrix_runtime().await;
        // Another edit moves the origin first; the later commit still lands its own value.
        let first = origin_ticket(&runtime, MatrixTransformField::OriginX, 3.0);
        fixture::settle_ticket(&runtime, &first).await;
        let second = origin_ticket(&runtime, MatrixTransformField::OriginX, 9.0);
        fixture::settle_ticket(&runtime, &second).await;
        assert!(matches!(second.settlement(true), Settlement::Landed { .. }));
        assert_eq!(fixture::accepted_matrix(&runtime).origin.x, 9.0);
        // Committing the value that is already accepted resolves Unchanged: it lands without
        // a new revision.
        let revision = runtime.model().accepted.unwrap().document.revision;
        let same = origin_ticket(&runtime, MatrixTransformField::OriginX, 9.0);
        fixture::settle_ticket(&runtime, &same).await;
        assert!(matches!(same.settlement(true), Settlement::Landed { .. }));
        assert_eq!(
            runtime.model().accepted.unwrap().document.revision,
            revision
        );
    }
}
