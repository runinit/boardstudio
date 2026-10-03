//! Definition-level model attachment and alignment in the selected Parts Inspector.

use super::{PartsSelection, PartsSelectionGeneration};
use crate::{
    operation_outcomes::OutcomeSlot,
    presentation::{SelectionAdapter, WorkspaceState, model_asset_import::read_model_file},
    runtime::Runtime,
};
use boardstudio_application::{AcceptedSnapshot, Event, Scope, SessionEpoch, TerminalOutcome};
use boardstudio_core::model::{
    Asset, EditCommand, EditOperation, EditPhase, PartDefinition, PartModel, Vec3,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelEditorOwner {
    scope: Option<Scope>,
    session_epoch: SessionEpoch,
    document_id: String,
    definition_id: String,
    selection_generation: u64,
    scope_generation: u64,
}

#[derive(Clone)]
struct ModelEditorMessage {
    owner: ModelEditorOwner,
    text: String,
}

#[derive(Clone)]
struct PendingModelEdit {
    owner: ModelEditorOwner,
    outcome: OutcomeSlot,
}

#[derive(Clone, Copy)]
enum VectorField {
    Offset,
    Rotation,
    Scale,
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

impl ModelEditorOwner {
    fn new(
        snapshot: &AcceptedSnapshot,
        scope: Option<Scope>,
        definition: &PartDefinition,
        selection_generation: u64,
        scope_generation: u64,
    ) -> Self {
        Self {
            scope,
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            definition_id: definition.id.clone(),
            selection_generation,
            scope_generation,
        }
    }
}

fn current_snapshot(
    runtime: &Runtime,
    owner: &ModelEditorOwner,
    selected: &PartsSelection,
    selection_generation: u64,
    scope_generation: u64,
    workspace: &Signal<&'static str>,
) -> Option<AcceptedSnapshot> {
    if workspace() != "Parts"
        || owner.scope.is_none()
        || runtime.scope() != owner.scope
        || owner.selection_generation != selection_generation
        || owner.scope_generation != scope_generation
        || selected() != Some((owner.scope.clone(), owner.definition_id.clone()))
    {
        return None;
    }
    let snapshot = runtime.model().accepted?;
    (snapshot.session_epoch == owner.session_epoch && snapshot.document.id == owner.document_id)
        .then_some(snapshot)
}

fn prepare_definition_edit(
    runtime: &Runtime,
    snapshot: &AcceptedSnapshot,
    owner: &ModelEditorOwner,
    selected: &PartsSelection,
    selection_generation: u64,
    scope_generation: u64,
    workspace: &Signal<&'static str>,
    original: &PartDefinition,
    project_owned_at_start: bool,
    update: impl FnOnce(
        &mut PartDefinition,
        &mut boardstudio_core::model::ProjectDoc,
    ) -> Result<(), String>,
    operation_id: boardstudio_application::OperationId,
) -> Result<Event, String> {
    let mut document = snapshot.document.as_ref().clone();
    let existing_index = document
        .definitions
        .iter()
        .position(|definition| definition.id == owner.definition_id);
    if project_owned_at_start && existing_index.is_none() {
        return Err(
            "This project component was removed. Re-select it before editing its model.".into(),
        );
    }
    let mut definition = existing_index
        .map(|index| document.definitions[index].clone())
        .unwrap_or_else(|| original.clone());
    if definition.id != original.id || definition.generator.is_some() {
        return Err("This selection is not a regular component definition.".into());
    }
    update(&mut definition, &mut document)?;
    if let Some(index) = existing_index {
        document.definitions[index] = definition;
    } else {
        if document
            .definitions
            .iter()
            .any(|item| item.id == definition.id)
        {
            return Err("The component definition identity is already in use.".into());
        }
        document.definitions.push(definition);
    }
    if current_snapshot(
        runtime,
        owner,
        selected,
        selection_generation,
        scope_generation,
        workspace,
    )
    .is_none()
    {
        return Err("The selected component changed before the model edit could be saved.".into());
    }
    Ok(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: snapshot.document.revision,
            transaction_id: format!("parts-component-model-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids: vec![owner.definition_id.clone()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        },
    })
}

fn initial_model(definition: &PartDefinition) -> Option<PartModel> {
    definition.models.as_ref()?.first().cloned()
}

fn vector_values(value: Vec3) -> [f64; 3] {
    [value.x, value.y, value.z]
}

fn model_vector(model: &PartModel, field: VectorField) -> Vec3 {
    match field {
        VectorField::Offset => model.offset,
        VectorField::Rotation => model.rotation,
        VectorField::Scale => model.scale,
    }
}

fn set_model_axis(model: &mut PartModel, field: VectorField, axis: Axis, value: f64) {
    let vector = match field {
        VectorField::Offset => &mut model.offset,
        VectorField::Rotation => &mut model.rotation,
        VectorField::Scale => &mut model.scale,
    };
    match axis {
        Axis::X => vector.x = value,
        Axis::Y => vector.y = value,
        Axis::Z => vector.z = value,
    }
}

fn submit_model_transform(
    runtime: &Rc<Runtime>,
    owner: &ModelEditorOwner,
    selected: &PartsSelection,
    selection_generation: u64,
    scope_generation: u64,
    workspace: &Signal<&'static str>,
    original: &PartDefinition,
    project_owned_at_start: bool,
    asset_id: &str,
    field: VectorField,
    axis: Axis,
    value: f64,
) -> Result<OutcomeSlot, String> {
    let snapshot = current_snapshot(
        runtime,
        owner,
        selected,
        selection_generation,
        scope_generation,
        workspace,
    )
    .ok_or_else(|| {
        "The selected component changed. Re-select it before editing its model.".to_owned()
    })?;
    let operation_id = runtime.operation();
    let event = prepare_definition_edit(
        runtime,
        &snapshot,
        owner,
        selected,
        selection_generation,
        scope_generation,
        workspace,
        original,
        project_owned_at_start,
        |definition, _document| {
            let models = definition
                .models
                .as_mut()
                .ok_or_else(|| "The attached model is no longer available.".to_owned())?;
            let model = models
                .first_mut()
                .filter(|model| model.asset_id == asset_id)
                .ok_or_else(|| {
                    "The attached model changed. Reopen its alignment controls.".to_owned()
                })?;
            if !value.is_finite() || matches!(field, VectorField::Scale) && value <= 0.0 {
                return Err(if matches!(field, VectorField::Scale) {
                    "Model scale must be a positive finite number.".into()
                } else {
                    "Model alignment values must be finite numbers.".into()
                });
            }
            set_model_axis(model, field, axis, value);
            Ok(())
        },
        operation_id,
    )?;
    let outcome = runtime.observe_operation(operation_id);
    runtime.submit(event);
    Ok(outcome)
}

fn submit_remove_model(
    runtime: &Rc<Runtime>,
    owner: &ModelEditorOwner,
    selected: &PartsSelection,
    selection_generation: u64,
    scope_generation: u64,
    workspace: &Signal<&'static str>,
    original: &PartDefinition,
    project_owned_at_start: bool,
    asset_id: &str,
) -> Result<OutcomeSlot, String> {
    let snapshot = current_snapshot(
        runtime,
        owner,
        selected,
        selection_generation,
        scope_generation,
        workspace,
    )
    .ok_or_else(|| {
        "The selected component changed. Re-select it before removing its model.".to_owned()
    })?;
    let operation_id = runtime.operation();
    let event = prepare_definition_edit(
        runtime,
        &snapshot,
        owner,
        selected,
        selection_generation,
        scope_generation,
        workspace,
        original,
        project_owned_at_start,
        |definition, _document| {
            let models = definition
                .models
                .as_mut()
                .ok_or_else(|| "The attached model is no longer available.".to_owned())?;
            if models
                .first()
                .is_none_or(|model| model.asset_id != asset_id)
            {
                return Err(
                    "The attached model changed. Reopen the Inspector before removing it.".into(),
                );
            }
            models.remove(0);
            if models.is_empty() {
                definition.models = None;
            }
            Ok(())
        },
        operation_id,
    )?;
    let outcome = runtime.observe_operation(operation_id);
    runtime.submit(event);
    Ok(outcome)
}

#[component]
pub(super) fn ComponentModelEditor(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    selected: PartsSelection,
    definition: PartDefinition,
    project_owned: bool,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let selection_generation = use_context::<PartsSelectionGeneration>().0;
    let scope_generation = use_context::<SelectionAdapter>().generation;
    let workspace = use_context::<WorkspaceState>().0;
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let owner = ModelEditorOwner::new(
        &snapshot,
        scope.clone(),
        &definition,
        selection_generation(),
        scope_generation(),
    );
    let binding = initial_model(&definition);
    let synced_binding = use_signal(|| binding.clone());
    let mut error = use_signal(|| None::<ModelEditorMessage>);
    let mut notice = use_signal(|| None::<ModelEditorMessage>);
    let mut uploading = use_signal(|| false);
    let mut pending = use_signal(|| None::<PendingModelEdit>);
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let request_generation = use_hook(|| Rc::new(Cell::new(0_u64)));
    use_drop({
        let alive = alive.clone();
        let request_generation = request_generation.clone();
        move || {
            alive.set(false);
            request_generation.set(request_generation.get().wrapping_add(1));
        }
    });
    use_effect(use_reactive((&binding,), {
        let mut synced_binding = synced_binding;
        let mut error = error;
        let mut notice = notice;
        move |(next,)| {
            if synced_binding() != next {
                synced_binding.set(next);
                error.set(None);
                notice.set(None);
            }
        }
    }));
    use_effect(use_reactive((&version(),), {
        let runtime = runtime.clone();
        let owner = owner.clone();
        let selected = selected;
        let selection_generation = selection_generation;
        let scope_generation = scope_generation;
        let workspace = workspace;
        let mut pending = pending;
        let mut error = error;
        let mut notice = notice;
        let definition_id = definition.id.clone();
        move |_| {
            let Some(waiting) = pending.read().clone() else {
                return;
            };
            let Some(outcome) = waiting.outcome.borrow().clone() else {
                if current_snapshot(
                    &runtime,
                    &waiting.owner,
                    &selected,
                    selection_generation(),
                    scope_generation(),
                    &workspace,
                )
                .is_none()
                {
                    pending.set(None);
                    error.set(None);
                    notice.set(None);
                }
                return;
            };
            pending.set(None);
            if waiting.owner != owner
                || current_snapshot(
                    &runtime,
                    &waiting.owner,
                    &selected,
                    selection_generation(),
                    scope_generation(),
                    &workspace,
                )
                .is_none()
                || waiting.owner.definition_id != definition_id
            {
                error.set(None);
                notice.set(None);
                return;
            }
            match outcome {
                TerminalOutcome::Completed => {
                    error.set(None);
                    notice.set(Some(ModelEditorMessage {
                        owner: waiting.owner,
                        text: "Model update accepted.".into(),
                    }));
                }
                TerminalOutcome::Rejected(message)
                | TerminalOutcome::PersistenceFailed(message)
                | TerminalOutcome::BlockedByRecovery(message)
                | TerminalOutcome::ExecutorFailed(message) => {
                    error.set(Some(ModelEditorMessage {
                        owner: waiting.owner,
                        text: message,
                    }));
                    notice.set(None);
                }
                TerminalOutcome::Cancelled
                | TerminalOutcome::Closed
                | TerminalOutcome::Superseded => {
                    error.set(None);
                    notice.set(None);
                }
            }
        }
    }));

    let current_owner = current_snapshot(
        &runtime,
        &owner,
        &selected,
        selection_generation(),
        scope_generation(),
        &workspace,
    );
    let visible_error = error()
        .filter(|message| message.owner == owner && current_owner.is_some())
        .map(|message| message.text);
    let visible_notice = notice()
        .filter(|message| message.owner == owner && current_owner.is_some())
        .map(|message| message.text);
    let busy = uploading() || pending().is_some();
    let base_definition = definition.clone();
    let selected_for_upload = selected;
    let runtime_for_upload = runtime.clone();
    let owner_for_upload = owner.clone();
    let upload_generation = request_generation.clone();
    let upload_model = move |event: FormEvent| {
        let Some(input) = event
            .data()
            .try_as_web_event()
            .and_then(|event| event.target())
            .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
        else {
            return;
        };
        let Some(file) = input.files().and_then(|files| files.get(0)) else {
            return;
        };
        input.set_value("");
        if busy
            || current_snapshot(
                &runtime_for_upload,
                &owner_for_upload,
                &selected_for_upload,
                selection_generation(),
                scope_generation(),
                &workspace,
            )
            .is_none()
        {
            return;
        }
        error.set(None);
        notice.set(None);
        uploading.set(true);
        let generation = request_generation.get().wrapping_add(1);
        request_generation.set(generation);
        let request_generation = request_generation.clone();
        let alive = alive.clone();
        let runtime = runtime_for_upload.clone();
        let owner = owner_for_upload.clone();
        let selected = selected_for_upload;
        let selection_generation = selection_generation;
        let scope_generation = scope_generation;
        let workspace = workspace;
        let original = base_definition.clone();
        let project_owned_at_start = project_owned;
        let mut uploading = uploading;
        let mut pending = pending;
        let mut error = error;
        let mut notice = notice;
        spawn_local(async move {
            let imported = read_model_file(file).await;
            if !alive.get() || request_generation.get() != generation {
                return;
            }
            let imported = match imported {
                Ok(imported) => imported,
                Err(message) => {
                    uploading.set(false);
                    if current_snapshot(
                        &runtime,
                        &owner,
                        &selected,
                        selection_generation(),
                        scope_generation(),
                        &workspace,
                    )
                    .is_some()
                    {
                        error.set(Some(ModelEditorMessage {
                            owner,
                            text: message,
                        }));
                    }
                    return;
                }
            };
            if current_snapshot(
                &runtime,
                &owner,
                &selected,
                selection_generation(),
                scope_generation(),
                &workspace,
            )
            .is_none()
            {
                uploading.set(false);
                return;
            }
            if let Err(message) = imported.store(&runtime.store).await {
                uploading.set(false);
                if current_snapshot(
                    &runtime,
                    &owner,
                    &selected,
                    selection_generation(),
                    scope_generation(),
                    &workspace,
                )
                .is_some()
                {
                    error.set(Some(ModelEditorMessage {
                        owner,
                        text: message,
                    }));
                }
                return;
            }
            if !alive.get() || request_generation.get() != generation {
                uploading.set(false);
                return;
            }
            let Some(snapshot) = current_snapshot(
                &runtime,
                &owner,
                &selected,
                selection_generation(),
                scope_generation(),
                &workspace,
            ) else {
                uploading.set(false);
                return;
            };
            let asset_id = loop {
                let candidate = format!("model-asset-{}", runtime.operation().0);
                if !snapshot
                    .document
                    .assets
                    .iter()
                    .any(|asset| asset.id == candidate)
                {
                    break candidate;
                }
            };
            let operation_id = runtime.operation();
            let asset = Asset {
                id: asset_id.clone(),
                name: imported.filename.clone(),
                media_type: imported.media_type.clone(),
                sha256: imported.sha256.clone(),
                license: None,
                source: Some("local file".into()),
            };
            let event = match prepare_definition_edit(
                &runtime,
                &snapshot,
                &owner,
                &selected,
                selection_generation(),
                scope_generation(),
                &workspace,
                &original,
                project_owned_at_start,
                |definition, document| {
                    let mut models = definition.models.clone().unwrap_or_default();
                    let new_model = PartModel {
                        asset_id: asset_id.clone(),
                        offset: Vec3::default(),
                        rotation: Vec3::default(),
                        scale: Vec3 {
                            x: 1.0,
                            y: 1.0,
                            z: 1.0,
                        },
                    };
                    if models.is_empty() {
                        models.push(new_model);
                    } else {
                        models[0] = new_model;
                    }
                    definition.models = Some(models);
                    document.assets.push(asset.clone());
                    Ok(())
                },
                operation_id,
            ) {
                Ok(event) => event,
                Err(message) => {
                    uploading.set(false);
                    if current_snapshot(
                        &runtime,
                        &owner,
                        &selected,
                        selection_generation(),
                        scope_generation(),
                        &workspace,
                    )
                    .is_some()
                    {
                        error.set(Some(ModelEditorMessage {
                            owner,
                            text: message,
                        }));
                    }
                    return;
                }
            };
            uploading.set(false);
            error.set(None);
            notice.set(None);
            pending.set(Some(PendingModelEdit {
                owner: owner.clone(),
                outcome: runtime.observe_operation(operation_id),
            }));
            runtime.submit(event);
        });
    };

    let cancel_upload = {
        let request_generation = request_generation.clone();
        let mut uploading = uploading;
        move |_| {
            request_generation.set(request_generation.get().wrapping_add(1));
            uploading.set(false);
        }
    };
    let binding = initial_model(&definition);
    let transform_commit = use_callback({
        let runtime_for_transform = runtime.clone();
        let owner = owner.clone();
        let original = definition.clone();
        let selected = selected;
        let workspace = workspace;
        let asset_id = binding.as_ref().map(|model| model.asset_id.clone());
        let mut pending = pending;
        let mut error = error;
        let mut notice = notice;
        move |(field, axis, value): (VectorField, Axis, f64)| {
            let Some(asset_id) = asset_id.as_deref() else {
                return;
            };
            if pending().is_some() {
                return;
            }
            match submit_model_transform(
                &runtime_for_transform,
                &owner,
                &selected,
                selection_generation(),
                scope_generation(),
                &workspace,
                &original,
                project_owned,
                asset_id,
                field,
                axis,
                value,
            ) {
                Ok(outcome) => {
                    error.set(None);
                    notice.set(None);
                    pending.set(Some(PendingModelEdit {
                        owner: owner.clone(),
                        outcome,
                    }));
                }
                Err(message) => error.set(Some(ModelEditorMessage {
                    owner: owner.clone(),
                    text: message,
                })),
            }
        }
    });
    let remove_model = {
        let runtime = runtime.clone();
        let owner = owner.clone();
        let original = definition.clone();
        let selected = selected;
        let workspace = workspace;
        let asset_id = binding.as_ref().map(|model| model.asset_id.clone());
        let mut pending = pending;
        let mut error = error;
        let mut notice = notice;
        move |_| {
            let Some(asset_id) = asset_id.as_deref() else {
                return;
            };
            if pending().is_some() {
                return;
            }
            match submit_remove_model(
                &runtime,
                &owner,
                &selected,
                selection_generation(),
                scope_generation(),
                &workspace,
                &original,
                project_owned,
                asset_id,
            ) {
                Ok(outcome) => {
                    error.set(None);
                    notice.set(None);
                    pending.set(Some(PendingModelEdit {
                        owner: owner.clone(),
                        outcome,
                    }));
                }
                Err(message) => error.set(Some(ModelEditorMessage {
                    owner: owner.clone(),
                    text: message,
                })),
            }
        }
    };
    let current_assets = current_owner
        .as_ref()
        .map(|snapshot| snapshot.document.assets.as_slice())
        .unwrap_or_default();
    let asset_name = binding
        .as_ref()
        .and_then(|model| {
            current_assets
                .iter()
                .find(|asset| asset.id == model.asset_id)
        })
        .map(|asset| asset.name.clone())
        .or_else(|| binding.as_ref().map(|model| model.asset_id.clone()));

    rsx! {
        details { class: "m1-generator-settings m1-component-model-editor",
            summary { "3D model" small { if binding.is_some() { "Attached" } else { "Optional" } } }
            if let Some(model) = binding.as_ref() {
                if let Some(name) = asset_name.as_ref() {
                p { class: "m1-parts-empty", "Bound asset: {name}" }
                }
                fieldset { disabled: busy,
                    legend { "Model alignment" }
                    ModelVectorEditor { title: "Offset", value: model.offset, unit: "mm", positive: false, on_commit: move |(axis, value)| transform_commit.call((VectorField::Offset, axis, value)) }
                    ModelVectorEditor { title: "Rotation", value: model.rotation, unit: "°", positive: false, on_commit: move |(axis, value)| transform_commit.call((VectorField::Rotation, axis, value)) }
                    ModelVectorEditor { title: "Scale", value: model.scale, unit: "×", positive: true, on_commit: move |(axis, value)| transform_commit.call((VectorField::Scale, axis, value)) }
                }
                button { r#type: "button", disabled: busy, onclick: remove_model, "Remove attached model" }
            } else {
                p { class: "m1-parts-empty", "No model attached to this component." }
            }
            label { class: "m1-generator-field", "Attach STEP / STL / WRL model",
                input { r#type: "file", accept: ".step,.stp,.stl,.wrl,model/step,model/stl,model/vrml", disabled: busy || current_owner.is_none(), onchange: upload_model }
            }
            if uploading() {
                p { class: "m1-parts-loading", role: "status", "Reading and saving model file…" }
                button { r#type: "button", onclick: cancel_upload, "Cancel model import" }
            }
            if pending().is_some() { p { class: "m1-parts-loading", role: "status", "Saving model change…" } }
            if let Some(message) = visible_notice { p { class: "m1-parts-preview-status", role: "status", "{message}" } }
            if let Some(message) = visible_error { p { class: "m1-parts-load-error", role: "alert", "{message}" } }
        }
    }
}

#[component]
fn ModelVectorEditor(
    title: &'static str,
    value: Vec3,
    unit: &'static str,
    positive: bool,
    on_commit: EventHandler<(Axis, f64)>,
) -> Element {
    let current = vector_values(value);
    let mut draft = use_signal(|| current.map(|coordinate| coordinate.to_string()));
    let mut synced = use_signal(|| current);
    let mut error = use_signal(|| None::<String>);
    use_effect(use_reactive((&current,), move |(next,)| {
        if synced() != next {
            synced.set(next);
            draft.set(next.map(|coordinate| coordinate.to_string()));
            error.set(None);
        }
    }));
    rsx! {
        fieldset { class: "m1-model-vector",
            legend { "{title} · {unit}" }
            for (index, axis, label) in [(0, Axis::X, "X"), (1, Axis::Y, "Y"), (2, Axis::Z, "Z")] {
                label { class: "m1-generator-field", "{label}",
                    input {
                        r#type: "number",
                        step: "0.1",
                        min: if positive { "0.001" } else { "" },
                        value: "{draft()[index]}",
                        aria_label: "{title} {label} {unit}",
                        aria_invalid: error().is_some(),
                        oninput: move |event| draft.with_mut(|values| values[index] = event.value()),
                        onblur: move |_| {
                            let raw = draft()[index].clone();
                            let parsed = raw.trim().parse::<f64>().ok();
                            let valid = parsed.is_some_and(|number| number.is_finite() && (!positive || number > 0.0));
                            if !valid {
                                error.set(Some(if positive { "Enter a positive finite number." } else { "Enter a finite number." }.into()));
                                return;
                            }
                            let number = parsed.unwrap_or_default();
                            error.set(None);
                            if number != current[index] { on_commit.call((axis, number)); }
                        },
                        onkeydown: move |event: KeyboardEvent| {
                            let key = event.key().to_string();
                            if key == "Enter"
                                && let Some(input) = event.data().try_as_web_event().and_then(|event| event.target()).and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                            {
                                let _ = input.blur();
                            } else if key == "Escape" {
                                draft.with_mut(|values| values[index] = current[index].to_string());
                                error.set(None);
                            }
                        }
                    }
                }
            }
            if let Some(message) = error() { small { role: "alert", "{message}" } }
        }
    }
}
