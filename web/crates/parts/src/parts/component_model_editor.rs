//! Definition-level model attachment and alignment in the selected Parts Inspector.

use super::{PartsSelection, PartsSelectionGeneration};
use crate::parts_custom_definition::replacement_commit;
use crate::{
    presentation::{SelectionAdapter, WorkspaceState, model_asset_import::read_model_file},
    runtime::Runtime,
};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SessionEpoch};
use boardstudio_core::model::{Asset, EditOperation, PartDefinition, PartModel, Vec3};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
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

/// This editor's bounded keys: the two one-shot actions plus one key per vector axis
/// field, so each axis observes its own edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelKey {
    Upload,
    Remove,
    Transform { field: VectorField, axis: Axis },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VectorField {
    Offset,
    Rotation,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

fn model_resolver(
    original: PartDefinition,
    project_owned_at_start: bool,
    update: impl Fn(&mut PartDefinition, &mut boardstudio_core::model::ProjectDoc) -> Result<(), String>
    + 'static,
) -> EditResolver {
    EditResolver::new(
        "parts-component-model",
        move |snapshot: &AcceptedSnapshot| {
            let result = (|| -> Result<_, String> {
                let mut document = snapshot.document.as_ref().clone();
                let existing_index = document
                    .definitions
                    .iter()
                    .position(|definition| definition.id == original.id);
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
                Ok(document)
            })();
            match result {
                Ok(document) if document == *snapshot.document => Resolution::Unchanged,
                Ok(document) => replacement_commit(
                    EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                    vec![original.id.clone()],
                ),
                Err(reason) => Resolution::Retire(reason),
            }
        },
    )
}

fn initial_model(definition: &PartDefinition) -> Option<PartModel> {
    definition.models.as_ref()?.first().cloned()
}

fn vector_values(value: Vec3) -> [f64; 3] {
    [value.x, value.y, value.z]
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
) -> Result<EditResolver, String> {
    current_snapshot(
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
    let asset_id = asset_id.to_owned();
    let resolver = model_resolver(
        original.clone(),
        project_owned_at_start,
        move |definition, _document| {
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
    );
    Ok(resolver)
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
) -> Result<EditResolver, String> {
    current_snapshot(
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
    let asset_id = asset_id.to_owned();
    let resolver = model_resolver(
        original.clone(),
        project_owned_at_start,
        move |definition, _document| {
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
    );
    Ok(resolver)
}

#[component]
pub fn ComponentModelEditor(
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
    let pending_edits = use_hook(|| PendingEditSignals::<ModelKey>::new());
    let submit_owner = use_signal(|| None::<ModelEditorOwner>);
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
        let pending_edits = pending_edits.clone();
        let submit_owner = submit_owner;
        let mut error = error;
        let mut notice = notice;
        move |_| {
            let Some(owner) = submit_owner.peek().clone() else {
                return;
            };
            let live = current_snapshot(
                &runtime,
                &owner,
                &selected,
                selection_generation(),
                scope_generation(),
                &workspace,
            )
            .is_some();
            let results = pending_edits.settle(live, |_| String::new());
            for result in results {
                if let PendingEditResult::Failed { message, .. } = result {
                    error.set(Some(ModelEditorMessage {
                        owner: owner.clone(),
                        text: message,
                    }));
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
    let upload_pending = pending_edits.is_pending(&ModelKey::Upload);
    let remove_pending = pending_edits.is_pending(&ModelKey::Remove);
    let base_definition = definition.clone();
    let selected_for_upload = selected;
    let runtime_for_upload = runtime.clone();
    let owner_for_upload = owner.clone();
    let upload_generation = request_generation.clone();
    let upload_pending_edits = pending_edits.clone();
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
        if uploading()
            || upload_pending
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
        let generation = upload_generation.get().wrapping_add(1);
        upload_generation.set(generation);
        let request_generation = upload_generation.clone();
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
        let pending_edits = upload_pending_edits.clone();
        let mut submit_owner = submit_owner;
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
            let Some(_snapshot) = current_snapshot(
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
            let asset_id = format!("model-asset-{}", runtime.operation().0);
            let asset = Asset {
                id: asset_id.clone(),
                name: imported.filename.clone(),
                media_type: imported.media_type.clone(),
                sha256: imported.sha256.clone(),
                license: None,
                source: Some("local file".into()),
            };
            let resolver = model_resolver(
                original,
                project_owned_at_start,
                move |definition, document| {
                    let mut asset = asset.clone();
                    let mut suffix = 0_u64;
                    while document
                        .assets
                        .iter()
                        .any(|existing| existing.id == asset.id)
                    {
                        suffix += 1;
                        asset.id = format!("{asset_id}-{suffix}");
                    }
                    let mut models = definition.models.clone().unwrap_or_default();
                    let new_model = PartModel {
                        asset_id: asset.id.clone(),
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
            );
            uploading.set(false);
            error.set(None);
            notice.set(None);
            pending_edits.begin_one_shot(
                &runtime,
                ModelKey::Upload,
                "parts-component-model",
                Some("component model".into()),
                resolver,
            );
            submit_owner.set(Some(owner));
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
        let runtime = runtime.clone();
        let owner = owner.clone();
        let original = definition.clone();
        let selected = selected;
        let workspace = workspace;
        let asset_id = binding.as_ref().map(|model| model.asset_id.clone());
        let mut error = error;
        let mut notice = notice;
        move |(field, axis, _submitted, value): (VectorField, Axis, String, f64)| {
            let Some(asset_id) = asset_id.as_deref() else {
                return None;
            };
            match submit_model_transform(
                &runtime,
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
                Ok(resolver) => {
                    error.set(None);
                    notice.set(None);
                    Some(resolver)
                }
                Err(message) => {
                    error.set(Some(ModelEditorMessage {
                        owner: owner.clone(),
                        text: message,
                    }));
                    None
                }
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
        let pending_edits = pending_edits.clone();
        let mut submit_owner = submit_owner;
        let mut error = error;
        let mut notice = notice;
        move |_| {
            let Some(asset_id) = asset_id.as_deref() else {
                return;
            };
            if pending_edits.is_pending(&ModelKey::Remove) {
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
                Ok(resolver) => {
                    error.set(None);
                    notice.set(None);
                    pending_edits.begin_one_shot(
                        &runtime,
                        ModelKey::Remove,
                        "parts-component-model",
                        Some("component model".into()),
                        resolver,
                    );
                    submit_owner.set(Some(owner.clone()));
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
                fieldset {
                    legend { "Model alignment" }
                    ModelVectorEditor { key: "{owner:?}-Offset", field: VectorField::Offset, title: "Offset", value: model.offset, unit: "mm", positive: false,
                        on_commit: move |(axis, submitted, value)| transform_commit.call((VectorField::Offset, axis, submitted, value)) }
                    ModelVectorEditor { key: "{owner:?}-Rotation", field: VectorField::Rotation, title: "Rotation", value: model.rotation, unit: "°", positive: false,
                        on_commit: move |(axis, submitted, value)| transform_commit.call((VectorField::Rotation, axis, submitted, value)) }
                    ModelVectorEditor { key: "{owner:?}-Scale", field: VectorField::Scale, title: "Scale", value: model.scale, unit: "×", positive: true,
                        on_commit: move |(axis, submitted, value)| transform_commit.call((VectorField::Scale, axis, submitted, value)) }
                }
                button { r#type: "button", disabled: remove_pending, onclick: remove_model, "Remove attached model" }
            } else {
                p { class: "m1-parts-empty", "No model attached to this component." }
            }
            label { class: "m1-generator-field", "Attach STEP / STL / WRL model",
                input { r#type: "file", accept: ".step,.stp,.stl,.wrl,model/step,model/stl,model/vrml", disabled: uploading() || upload_pending || current_owner.is_none(), onchange: upload_model }
            }
            if uploading() {
                p { class: "m1-parts-loading", role: "status", "Reading and saving model file…" }
                button { r#type: "button", onclick: cancel_upload, "Cancel model import" }
            }

            if let Some(message) = visible_notice { p { class: "m1-parts-preview-status", role: "status", "{message}" } }
            if let Some(message) = visible_error { p { class: "m1-parts-load-error", role: "alert", "{message}" } }
        }
    }
}

#[component]
fn ModelVectorEditor(
    field: VectorField,
    title: &'static str,
    value: Vec3,
    unit: &'static str,
    positive: bool,
    on_commit: Callback<(Axis, String, f64), Option<EditResolver>>,
) -> Element {
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let runtime = use_context::<Rc<Runtime>>();
    // This editor's own collection: the editor is remounted per owner (its parent keys
    // it by the model owner), so its lifetime is the owner lifetime and it settles its
    // own axes here, like the render-body settlement it replaces.
    let pending = use_hook(|| PendingEditSignals::<ModelKey>::new());
    let axes = [Axis::X, Axis::Y, Axis::Z];
    let current = vector_values(value);
    let mut drafts = use_hook(|| current.map(|coordinate| Signal::new(coordinate.to_string())));
    let mut dirty = use_signal(|| [false; 3]);
    let mut synced = use_signal(|| current);
    let mut error = use_signal(|| None::<String>);
    let failure = use_signal(|| None::<String>);
    // Admission memory: the text each axis edit was submitted with, so a blur on an
    // untouched draft does not resubmit while the edit is pending.
    let mut submitted = use_signal(|| [None::<String>, None, None]);
    for index in 0..3 {
        pending.bind_field(
            ModelKey::Transform {
                field,
                axis: axes[index],
            },
            drafts[index],
            failure,
        );
    }
    {
        let previous = synced();
        for index in 0..3 {
            let key = ModelKey::Transform {
                field,
                axis: axes[index],
            };
            if !pending.is_pending(&key)
                && !dirty.peek()[index]
                && current[index] != previous[index]
            {
                drafts[index].set(current[index].to_string());
            }
        }
        if previous != current {
            synced.set(current);
        }
    }
    // Settle this editor's axes before rendering: a failure restores the accepted
    // value and reports inline; landed and retired edits drop silently.
    let settled = pending.settle(true, |key| match key {
        ModelKey::Transform { axis, .. } => match axis {
            Axis::X => value.x,
            Axis::Y => value.y,
            Axis::Z => value.z,
        }
        .to_string(),
        _ => String::new(),
    });
    for result in settled {
        let key = match result {
            PendingEditResult::Landed { key, .. }
            | PendingEditResult::Failed { key, .. }
            | PendingEditResult::Retired { key } => key,
        };
        if let ModelKey::Transform { axis, .. } = key {
            let index = match axis {
                Axis::X => 0,
                Axis::Y => 1,
                Axis::Z => 2,
            };
            // Blur admission lasts only as long as the latest axis edit is pending.
            submitted.write()[index] = None;
            if drafts[index].peek().as_str() == current[index].to_string() {
                dirty.write()[index] = false;
            }
        }
    }
    rsx! {
        fieldset { class: "m1-model-vector",
            legend { "{title} · {unit}" }
            for (index, axis, label) in [(0, Axis::X, "X"), (1, Axis::Y, "Y"), (2, Axis::Z, "Z")] {
                {
                    let pending = pending.clone();
                    let pending_for_escape = pending.clone();
                    let runtime = runtime.clone();
                    rsx! {
                    label { class: "m1-generator-field", "{label}",
                    input {
                        r#type: "number",
                        step: "0.1",
                        min: if positive { "0.001" } else { "" },
                        value: "{drafts[index]()}",
                        aria_label: "{title} {label} {unit}",
                        aria_invalid: error().is_some(),
                        oninput: move |event| {
                            drafts[index].set(event.value());
                            // Typing remains a draft even when it returns to the old
                            // accepted value while an earlier edit is still settling.
                            dirty.write()[index] = true;
                        },
                        onblur: move |_| {
                            let raw = drafts[index]().clone();
                            let parsed = raw.trim().parse::<f64>().ok();
                            let valid = parsed.is_some_and(|number| number.is_finite() && (!positive || number > 0.0));
                            if !valid {
                                error.set(Some(if positive { "Enter a positive finite number." } else { "Enter a finite number." }.into()));
                                return;
                            }
                            let number = parsed.unwrap_or_default();
                            error.set(None);
                            let already_submitted = submitted.peek()[index].as_deref() == Some(raw.as_str());
                            let axis_pending = pending.is_pending(&ModelKey::Transform { field, axis });
                            if !axis_pending && number == current[index] {
                                dirty.write()[index] = false;
                                return;
                            }
                            if !already_submitted {
                                if let Some(resolver) = on_commit.call((axis, raw.clone(), number)) {
                                    // The helper remembers the submitted axis text, so an
                                    // older outcome can never clobber a newer draft.
                                    pending.begin_field(
                                        &runtime,
                                        ModelKey::Transform { field, axis },
                                        "parts-component-model",
                                        Some("component model".into()),
                                        resolver,
                                        &raw,
                                    );
                                    submitted.write()[index] = Some(raw);
                                }
                            }
                        },
                        onkeydown: move |event: KeyboardEvent| {
                            let key = event.key().to_string();
                            if key == "Enter"
                                && let Some(input) = event.data().try_as_web_event().and_then(|event| event.target()).and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                            {
                                let _ = input.blur();
                            } else if key == "Escape" {
                                let restored = if pending_for_escape.is_pending(&ModelKey::Transform { field, axis }) {
                                    submitted.peek()[index].clone().unwrap_or_else(|| current[index].to_string())
                                } else {
                                    current[index].to_string()
                                };
                                drafts[index].set(restored);
                                dirty.write()[index] = false;
                                error.set(None);
                            }
                        }
                    }
                    }
                }
                }
            }
            if let Some(message) = error() { small { role: "alert", "{message}" } }
            if let Some(message) = failure() { small { role: "alert", "{message}" } }
        }
    }
}

#[cfg(test)]
mod settlement_tests {
    use super::*;
    use boardstudio_core::model::ProjectDoc;
    use boardstudio_web_runtime::pending_edits::PendingEdits;
    use boardstudio_web_runtime::runtime::project_name_test_support as support;
    use wasm_bindgen_test::*;

    fn definition() -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id":"component", "name":"Component", "kind":"custom", "courtyard":[], "pads":[],
            "models":[{"assetId":"model", "offset":{"x":0,"y":0,"z":0}, "rotation":{"x":0,"y":0,"z":0}, "scale":{"x":1,"y":1,"z":1}}]
        })).unwrap()
    }

    async fn opened() -> Rc<Runtime> {
        let runtime = support::new_runtime();
        let mut document = ProjectDoc::empty("model-edits", "Model edits");
        document.definitions.push(definition());
        document.assets.push(Asset {
            id: "model".into(),
            name: "Model".into(),
            media_type: "model/stl".into(),
            sha256: "a".repeat(64),
            license: None,
            source: None,
        });
        support::open_document(&runtime, document).await;
        runtime
    }

    fn offset_edit(axis: Axis, value: f64) -> EditResolver {
        model_resolver(definition(), true, move |definition, _| {
            let model = definition
                .models
                .as_mut()
                .and_then(|models| models.first_mut())
                .ok_or("The attached model is no longer available.")?;
            set_model_axis(model, VectorField::Offset, axis, value);
            Ok(())
        })
    }

    fn host() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        let _ = version();
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let accepted = runtime.model().accepted.unwrap();
        let model = initial_model(&accepted.document.definitions[0]).unwrap();
        rsx! { ModelVectorEditor { field: VectorField::Offset, title: "Offset", value: model.offset, unit: "mm", positive: false,
            on_commit: move |(axis, _submitted, value)| Some(offset_edit(axis, value))
        } }
    }

    async fn tick() {
        gloo_timers::future::TimeoutFuture::new(30).await;
    }
    fn input(root: &web_sys::Element, axis: &str) -> HtmlInputElement {
        root.query_selector(&format!("input[aria-label='Offset {axis} mm']"))
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }
    fn type_value(input: &HtmlInputElement, value: &str) {
        input.set_value(value);
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
            .unwrap();
    }
    async fn commit(input: &HtmlInputElement, value: &str) {
        input.focus().unwrap();
        type_value(input, value);
        tick().await;
        input.blur().unwrap();
        tick().await;
    }

    #[wasm_bindgen_test]
    async fn mounted_model_axis_can_commit_the_same_text_after_undo() {
        let runtime = opened().await;
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        tick().await;
        let x = input(&root, "X");
        commit(&x, "4").await;
        support::run_pending(&runtime).await;
        tick().await;
        assert_eq!(
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x,
            4.0,
            "the first model alignment edit lands"
        );
        runtime.submit(boardstudio_application::Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        tick().await;
        assert_eq!(x.value(), "0", "Undo restores the accepted axis value");

        commit(&x, "4").await;
        support::run_pending(&runtime).await;
        tick().await;
        assert_eq!(
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x,
            4.0,
            "the same typed text can land again after Undo"
        );
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_model_axis_preserves_newer_baseline_typing_when_edit_lands() {
        let runtime = opened().await;
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        tick().await;
        let x = input(&root, "X");
        let (entered, release) = support::gate_next_core_reply(&runtime);
        commit(&x, "4").await;
        support::drive_pending(&runtime);
        entered.await.unwrap();
        x.focus().unwrap();
        type_value(&x, "0");
        tick().await;
        release.send(()).unwrap();
        tick().await;
        support::run_pending(&runtime).await;
        tick().await;
        assert_eq!(
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x,
            4.0,
            "the older committed axis value lands"
        );
        assert_eq!(
            x.value(),
            "0",
            "new typing at the previous accepted baseline survives the older landing"
        );

        x.blur().unwrap();
        tick().await;
        support::run_pending(&runtime).await;
        tick().await;
        assert_eq!(
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x,
            0.0,
            "the preserved baseline draft can commit after the older edit lands"
        );
        runtime.submit(boardstudio_application::Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        tick().await;
        assert_eq!(x.value(), "4", "the clean axis follows Undo");
        runtime.submit(boardstudio_application::Event::Redo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        tick().await;
        assert_eq!(x.value(), "0", "the clean axis follows Redo");
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_model_axes_queue_preserve_typing_and_undo_in_order() {
        let runtime = opened().await;
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        tick().await;
        let x = input(&root, "X");
        let y = input(&root, "Y");
        let (entered, release) = support::gate_next_core_reply(&runtime);
        commit(&x, "4").await;
        support::drive_pending(&runtime);
        entered.await.unwrap();
        commit(&y, "7").await;
        type_value(&y, "9");
        tick().await;
        release.send(()).unwrap();
        tick().await;
        support::run_pending(&runtime).await;
        tick().await;
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            initial_model(&accepted.document.definitions[0])
                .unwrap()
                .offset,
            Vec3 {
                x: 4.0,
                y: 7.0,
                z: 0.0
            }
        );
        assert_eq!(x.value(), "4");
        assert_eq!(y.value(), "9", "new typing survives an earlier landing");
        runtime.submit(boardstudio_application::Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        tick().await;
        assert_eq!(
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset,
            Vec3 {
                x: 4.0,
                y: 0.0,
                z: 0.0
            }
        );
        runtime.submit(boardstudio_application::Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        tick().await;
        assert_eq!(
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset,
            Vec3::default()
        );
        let (entered, release) = support::gate_next_core_reply(&runtime);
        commit(&x, "7").await;
        support::drive_pending(&runtime);
        entered.await.unwrap();
        x.focus().unwrap();
        type_value(&x, "8");
        tick().await;
        let escape = web_sys::KeyboardEventInit::new();
        escape.set_key("Escape");
        escape.set_bubbles(true);
        x.dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap(),
        )
        .unwrap();
        tick().await;
        x.blur().unwrap();
        tick().await;
        release.send(()).unwrap();
        tick().await;
        support::run_pending(&runtime).await;
        tick().await;
        assert_eq!(
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x,
            7.0,
            "Escape cancels typing without reversing the preceding committed edit"
        );
        runtime.submit(boardstudio_application::Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        tick().await;
        support::fail_next_core_reply(&runtime, "controlled model failure");
        commit(&x, "12").await;
        support::run_pending(&runtime).await;
        tick().await;
        assert_eq!(
            x.value(),
            "0",
            "failed committed text returns to accepted value"
        );
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn model_edit_retires_if_definition_deleted_before_execution() {
        let runtime = opened().await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let mut edits = PendingEdits::default();
        edits.begin(
            &runtime,
            0_u64,
            "delete-definition-test",
            None,
            EditResolver::new("delete-definition-test", |accepted: &AcceptedSnapshot| {
                let mut document = accepted.document.as_ref().clone();
                document.definitions.clear();
                replacement_commit(
                    EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                    vec!["component".into()],
                )
            }),
        );
        support::drive_pending(&runtime);
        entered.await.unwrap();
        edits.begin(
            &runtime,
            1_u64,
            "model-test",
            Some("component model".into()),
            offset_edit(Axis::X, 4.0),
        );
        release.send(()).unwrap();
        tick().await;
        support::run_pending(&runtime).await;
        let results = edits.settle(true);
        assert!(
            matches!(&results[..], [PendingEditResult::Landed { .. }, PendingEditResult::Failed { message, .. }] if message.contains("removed")),
            "the deletion lands and the queued model edit fails against the removed definition"
        );
        assert!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .definitions
                .is_empty()
        );
    }
}
