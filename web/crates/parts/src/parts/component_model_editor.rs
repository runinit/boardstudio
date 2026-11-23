//! Definition-level model attachment and alignment in the selected Parts Inspector.

use super::{PartsSelection, PartsSelectionGeneration};
use crate::parts_custom_definition::replacement_commit;
use crate::{
    presentation::{SelectionAdapter, WorkspaceState, model_asset_import::read_model_file},
    runtime::Runtime,
};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SessionEpoch};
use boardstudio_model::model::{Asset, EditOperation, PartDefinition, PartModel, Vec3};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::NumericField;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonVariant},
    callout::{Callout, CalloutVariant},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    spinner::Spinner,
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
    workspace: &Signal<Workspace>,
) -> Option<AcceptedSnapshot> {
    if workspace() != Workspace::Parts
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
    update: impl Fn(
        &mut PartDefinition,
        &mut boardstudio_model::model::ProjectDoc,
    ) -> Result<(), String>
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
    live: bool,
    original: &PartDefinition,
    project_owned_at_start: bool,
    asset_id: &str,
    field: VectorField,
    axis: Axis,
    value: f64,
) -> Result<EditResolver, String> {
    if !live {
        return Err(
            "The selected component changed. Re-select it before editing its model.".to_owned(),
        );
    }
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
    live: bool,
    original: &PartDefinition,
    project_owned_at_start: bool,
    asset_id: &str,
) -> Result<EditResolver, String> {
    if !live {
        return Err(
            "The selected component changed. Re-select it before removing its model.".to_owned(),
        );
    }
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
    let mut file_input = use_signal(|| None::<HtmlInputElement>);
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
        if uploading()
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
            if let Err(text) = runtime.commit("parts-component-model", resolver) {
                error.set(Some(ModelEditorMessage { owner, text }));
            }
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
        let asset_id = binding.as_ref().map(|model| model.asset_id.clone());
        let mut error = error;
        let mut notice = notice;
        move |(field, axis, _submitted, value): (VectorField, Axis, String, f64)| {
            let asset_id = asset_id.as_deref()?;
            let live = current_snapshot(
                &runtime,
                &owner,
                &selected,
                selection_generation(),
                scope_generation(),
                &workspace,
            )
            .is_some();
            match submit_model_transform(
                live,
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
        let asset_id = binding.as_ref().map(|model| model.asset_id.clone());
        let mut error = error;
        let mut notice = notice;
        move |_| {
            let Some(asset_id) = asset_id.as_deref() else {
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
            match submit_remove_model(live, &original, project_owned, asset_id) {
                Ok(resolver) => {
                    error.set(None);
                    notice.set(None);
                    if let Err(text) = runtime.commit("parts-component-model", resolver) {
                        error.set(Some(ModelEditorMessage {
                            owner: owner.clone(),
                            text,
                        }));
                    }
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
        Collapsible { keep_mounted: true, default_open: false,
            CollapsibleTrigger { class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", aria_label: if binding.is_some() { "3D model Attached" } else { "3D model Optional" }, "3D model" small { if binding.is_some() { "Attached" } else { "Optional" } } }
            CollapsibleContent { class: "data-[open=false]:hidden grid gap-2",
                if let Some(model) = binding.as_ref() {
                    if let Some(name) = asset_name.as_ref() {
                    p { "Bound asset: {name}" }
                    }
                    fieldset { class: "grid gap-2",
                        legend { "Model alignment" }
                        { rsx! { ModelVectorEditor { key: "{owner:?}-Offset", title: "Offset", value: model.offset, unit: "mm", positive: false,
                            on_commit: move |(axis, submitted, value)| transform_commit.call((VectorField::Offset, axis, submitted, value)) } } }
                        { rsx! { ModelVectorEditor { key: "{owner:?}-Rotation", title: "Rotation", value: model.rotation, unit: "°", positive: false,
                            on_commit: move |(axis, submitted, value)| transform_commit.call((VectorField::Rotation, axis, submitted, value)) } } }
                        { rsx! { ModelVectorEditor { key: "{owner:?}-Scale", title: "Scale", value: model.scale, unit: "×", positive: true,
                            on_commit: move |(axis, submitted, value)| transform_commit.call((VectorField::Scale, axis, submitted, value)) } } }
                    }
                    Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", onclick: remove_model, "Remove attached model" }
                } else {
                    p { "No model attached to this component." }
                }
                Button {
                    variant: ButtonVariant::Secondary,
                    class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: uploading() || current_owner.is_none(),
                    onclick: move |_| {
                        if let Some(input) = file_input() {
                            input.click();
                        }
                    },
                    "Attach STEP / STL / WRL model"
                }
                input {
                    r#type: "file",
                    hidden: true,
                    accept: ".step,.stp,.stl,.wrl,model/step,model/stl,model/vrml",
                    aria_label: "Attach STEP / STL / WRL model",
                    disabled: uploading() || current_owner.is_none(),
                    onmounted: move |event: MountedEvent| {
                        file_input.set(event.data().downcast::<web_sys::Element>().and_then(|e| e.dyn_ref::<HtmlInputElement>()).cloned());
                    },
                    onchange: upload_model,
                }
                if uploading() {
                    div { role: "status", class: "flex flex-wrap items-center gap-2",
                        Spinner {}
                        "Reading and saving model file…"
                        Button { variant: ButtonVariant::Secondary, onclick: cancel_upload, "Cancel model import" }
                    }
                }

                if let Some(message) = visible_notice { div { role: "status", Callout { variant: CalloutVariant::Info, "{message}" } } }
                if let Some(message) = visible_error { Alert { variant: AlertVariant::Destructive, "{message}" } }
            }
        }
    }
}

#[component]
fn ModelVectorEditor(
    title: &'static str,
    value: Vec3,
    unit: &'static str,
    positive: bool,
    on_commit: Callback<(Axis, String, f64), Option<EditResolver>>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let current = vector_values(value);
    let mut error = use_signal(|| None::<String>);
    rsx! {
        fieldset { class: "grid gap-2",
            legend { "{title} · {unit}" }
            div { class: "grid grid-cols-3 gap-2",
                for (index, axis, label) in [(0, Axis::X, "X"), (1, Axis::Y, "Y"), (2, Axis::Z, "Z")] {
                    NumericField { key: "{label}", label, aria_label: format!("{title} {label} {unit}"),
                        value: current[index], min: positive.then_some(0.001),
                        on_commit: {
                            let runtime = runtime.clone();
                            move |number: f64| {
                                if let Some(resolver) = on_commit.call((axis, number.to_string(), number)) {
                                    error.set(runtime.commit("parts-component-model", resolver).err());
                                }
                            }
                        },
                    }
                }
            }
            if let Some(message) = error() {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            }
        }
    }
}
