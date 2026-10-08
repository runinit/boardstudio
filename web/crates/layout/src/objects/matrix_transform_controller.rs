//! Accepted-snapshot projection and exact operation lifecycle for transform fields.
use super::matrix_transform_inspector::{
    MatrixTransformFeedback, MatrixTransformInspectorOwner, MatrixTransformProjection,
    MatrixTransformRequest,
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
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
struct TransformPendingKey {
    field: MatrixTransformField,
    owner: MatrixTransformInspectorOwner,
    request_id: u64,
    one_shot: bool,
    failure: Option<Signal<Option<String>>>,
}

impl PartialEq for TransformPendingKey {
    fn eq(&self, other: &Self) -> bool {
        self.field == other.field && self.one_shot == other.one_shot
    }
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
    pub(super) inspector_mounted: Signal<bool>,
    pub(super) numeric_fields: Vec<(MatrixTransformField, Signal<String>, Signal<Option<String>>)>,
    one_shot_fields: Vec<(MatrixTransformField, Signal<bool>)>,
}

impl MatrixTransformInspectorMount {
    pub(super) fn one_shot_disabled(&self, field: MatrixTransformField) -> Signal<bool> {
        self.one_shot_fields
            .iter()
            .find(|(candidate, _)| *candidate == field)
            .expect("one-shot transform action has a stable view")
            .1
    }

    pub(super) fn numeric_field(
        &self,
        field: MatrixTransformField,
    ) -> (Signal<String>, Signal<Option<String>>) {
        let (_, draft, failure) = self
            .numeric_fields
            .iter()
            .find(|(candidate, _, _)| *candidate == field)
            .expect("numeric transform field has a stable view");
        (*draft, *failure)
    }

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
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot: false,
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

    let numeric_fields = use_hook(|| {
        [
            MatrixTransformField::OriginX,
            MatrixTransformField::OriginY,
            MatrixTransformField::MatrixRotation,
            MatrixTransformField::RowOffsetX,
            MatrixTransformField::RowOffsetY,
            MatrixTransformField::ColumnSplay,
            MatrixTransformField::SplayOriginX,
            MatrixTransformField::SplayOriginY,
            MatrixTransformField::ColumnStagger,
            MatrixTransformField::ColumnOffsetX,
            MatrixTransformField::ColumnOffsetY,
            MatrixTransformField::KeyOffsetX,
            MatrixTransformField::KeyOffsetY,
            MatrixTransformField::KeyRotation,
        ]
        .into_iter()
        .map(|field| {
            (
                field,
                Signal::new(String::new()),
                Signal::new(None::<String>),
            )
        })
        .collect::<Vec<_>>()
    });
    let request_sequence = use_signal(|| 0u64);
    let last_request_id = use_signal(|| 0u64);
    let pending = use_hook(|| PendingEditSignals::<TransformPendingKey>::new());
    let one_shot_fields = use_hook(|| {
        [
            MatrixTransformField::RowOffsetReset,
            MatrixTransformField::ColumnOffsetReset,
            MatrixTransformField::KeyTransformReset,
            MatrixTransformField::KeyAssembliesLocal,
            MatrixTransformField::KeyAttached,
        ]
        .into_iter()
        .map(|field| (field, Signal::new(false)))
        .collect::<Vec<_>>()
    });
    let feedback = use_signal(Vec::<MatrixTransformFeedback>::new);
    let alive = use_hook(|| Rc::new(std::cell::Cell::new(true)));
    let inspector_mounted = use_signal(|| false);
    use_drop({
        let alive = alive.clone();
        let pending = pending.clone();
        move || {
            alive.set(false);
            pending.settle(false, |_| String::new());
        }
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
            let mut feedback = feedback;
            let pending = pending.clone();
            let alive = alive.clone();
            let inspector_mounted = inspector_mounted;
            move |(_, current_workspace, _, _)| {
                let owner_is_live = alive.get()
                    && inspector_mounted()
                    && workspace_route_matches(current_workspace, owner_workspace);
                settle_pending(&runtime, &pending, owner_is_live, &mut feedback);
            }
        },
    ));

    let on_edit = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        let pending = pending.clone();
        let one_shot_fields = one_shot_fields.clone();
        let mut last_request_id = last_request_id;
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
            let key = TransformPendingKey {
                field: request.field,
                owner: request.owner.clone(),
                request_id: request.request_id,
                one_shot: request.one_shot,
                failure: request.failure,
            };
            if request.one_shot && pending.is_pending(&key) {
                return;
            }
            if !editable {
                publish_request_failure(
                    &mut feedback,
                    &request,
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
                publish_request_failure(&mut feedback, &request, Some(message.into()));
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
                publish_request_failure(&mut feedback, &request, Some(message));
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
            let resolver = transform_edit_resolver(
                request.owner.matrix_id.clone(),
                request.owner.scope.board_id.clone(),
                request.owner.context.clone(),
                request.field,
                request.value.clone(),
                request.splay_affect.clone(),
                catalogue_definitions,
            );
            if let (Some(draft), Some(failure)) = (request.draft, request.failure) {
                pending.bind_field(key.clone(), draft, failure);
            }
            if !request.one_shot {
                pending.begin_field(
                    &runtime,
                    key,
                    "layout-matrix-transform",
                    Some("transform".into()),
                    resolver,
                    request.submitted_text.as_deref().unwrap_or_default(),
                );
            } else {
                let Some((_, disabled)) = one_shot_fields
                    .iter()
                    .find(|(field, _)| *field == request.field)
                else {
                    return;
                };
                pending.bind_one_shot(key.clone(), *disabled);
                pending.begin_one_shot(
                    &runtime,
                    key,
                    "layout-matrix-transform",
                    Some("transform".into()),
                    resolver,
                );
            }
        }
    });

    MatrixTransformInspectorMount {
        projection,
        request_sequence,
        editable,
        feedback: feedback.read().clone(),
        on_edit,
        splay_affect,
        inspector_mounted,
        numeric_fields,
        one_shot_fields,
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
    pending: &PendingEditSignals<TransformPendingKey>,
    owner_is_live: bool,
    feedback: &mut Signal<Vec<MatrixTransformFeedback>>,
) {
    for result in pending.settle(owner_is_live, |key| accepted_field_text(runtime, key)) {
        match result {
            PendingEditResult::Landed { key, .. } => {
                if key.failure.is_none() {
                    publish_feedback(feedback, &key.owner, key.request_id, key.field, None)
                }
            }
            PendingEditResult::Failed { key, message } => {
                if key.failure.is_none() {
                    publish_feedback(
                        feedback,
                        &key.owner,
                        key.request_id,
                        key.field,
                        Some(message),
                    )
                }
            }
            PendingEditResult::Retired { key } => {
                clear_feedback(feedback, &key);
            }
        }
    }
}

fn accepted_field_text(runtime: &Runtime, key: &TransformPendingKey) -> String {
    let Some(snapshot) = runtime.model().accepted else {
        return String::new();
    };
    let Some(matrix) = snapshot
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == key.owner.matrix_id)
    else {
        return String::new();
    };
    transform_fields(
        &snapshot,
        &key.owner.scope.board_id,
        &key.owner.context,
        matrix,
    )
    .and_then(|fields| field_value(&fields, key.field))
    .and_then(|value| match value {
        MatrixTransformValue::Number(value) => Some(value.to_string()),
        _ => None,
    })
    .unwrap_or_default()
}

fn publish_request_failure(
    feedback: &mut Signal<Vec<MatrixTransformFeedback>>,
    request: &MatrixTransformRequest,
    message: Option<String>,
) {
    if let Some(mut failure) = request.failure {
        failure.set(message);
    } else {
        publish_feedback(
            feedback,
            &request.owner,
            request.request_id,
            request.field,
            message,
        );
    }
}

fn clear_feedback(feedback: &mut Signal<Vec<MatrixTransformFeedback>>, key: &TransformPendingKey) {
    let mut entries = feedback.read().clone();
    entries.retain(|entry| {
        entry.owner != key.owner || entry.field != key.field || entry.request_id != key.request_id
    });
    feedback.set(entries);
}

fn publish_feedback(
    feedback: &mut Signal<Vec<MatrixTransformFeedback>>,
    owner: &MatrixTransformInspectorOwner,
    request_id: u64,
    field: MatrixTransformField,
    message: Option<String>,
) {
    let next = MatrixTransformFeedback {
        owner: owner.clone(),
        request_id,
        field,
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
        let Some(index) = entries.iter().position(|entry| entry.message.is_some()) else {
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
    use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
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
