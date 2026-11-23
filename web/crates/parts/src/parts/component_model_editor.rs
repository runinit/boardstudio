//! Definition-level model attachment and alignment in the selected Parts Inspector.

use super::model_transform::{
    ModelAxis, ModelTransformFieldset, ModelVector, new_model, set_model_axis, upload_model_asset,
    valid_transform_value,
};
use super::{PartsSelection, PartsSelectionGeneration};
use crate::parts_custom_definition::{
    PartsOwnerGuard, replace_document_resolver, upsert_definition,
};
use crate::{
    presentation::{SelectionAdapter, WorkspaceState},
    runtime::Runtime,
};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Scope};
use boardstudio_model::model::{PartDefinition, PartModel};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::{FilePickerButton, InspectorSection, TOUCH_CLS};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonVariant},
    callout::{Callout, CalloutVariant},
    spinner::Spinner,
};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
struct ModelEditorMessage {
    owner: PartsOwnerGuard,
    text: String,
}

#[derive(Clone, Copy)]
enum ModelEditIntent {
    Transform(ModelVector, ModelAxis, f64),
    Remove,
}

fn current_snapshot(
    runtime: &Runtime,
    owner: &PartsOwnerGuard,
    selected: &PartsSelection,
    selection_generation: u64,
    scope_generation: u64,
    workspace: &Signal<Workspace>,
) -> Option<AcceptedSnapshot> {
    let snapshot = runtime.model().accepted?;
    owner
        .is_current(
            &snapshot,
            runtime.scope(),
            selected(),
            Some(selection_generation),
            Some(scope_generation),
            Some(workspace()),
        )
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
    replace_document_resolver("parts-component-model", move |_, document| {
        let existing = document
            .definitions
            .iter()
            .find(|definition| definition.id == original.id)
            .cloned();
        if project_owned_at_start && existing.is_none() {
            return Err(
                "This project component was removed. Re-select it before editing its model.".into(),
            );
        }
        let mut definition = existing.unwrap_or_else(|| original.clone());
        if definition.id != original.id || definition.generator.is_some() {
            return Err("This selection is not a regular component definition.".into());
        }
        let target = vec![original.id.clone()];
        update(&mut definition, document)?;
        upsert_definition(document, definition);
        Ok(target)
    })
}

fn initial_model(definition: &PartDefinition) -> Option<PartModel> {
    definition.models.as_ref()?.first().cloned()
}

fn submit_model_transform(
    live: bool,
    original: &PartDefinition,
    project_owned_at_start: bool,
    asset_id: &str,
    field: ModelVector,
    axis: ModelAxis,
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
            if !valid_transform_value(field, value) {
                return Err(if field == ModelVector::Scale {
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

/// Commit one model resolver and settle a commit-time failure onto the
/// owner-gated error, shared by the upload, transform and remove paths.
fn settle_model_commit(
    runtime: &Rc<Runtime>,
    owner: PartsOwnerGuard,
    mut error: Signal<Option<ModelEditorMessage>>,
    resolver: EditResolver,
) {
    if let Err(text) = runtime.commit("parts-component-model", resolver) {
        error.set(Some(ModelEditorMessage { owner, text }));
    }
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
    let owner = PartsOwnerGuard::capture(&snapshot, scope.clone(), &definition.id)
        .require_scope()
        .generations(Some(selection_generation()), Some(scope_generation()));
    let binding = initial_model(&definition);
    let synced_binding = use_signal(|| binding.clone());
    let mut error = use_signal(|| None::<ModelEditorMessage>);
    let mut notice = use_signal(|| None::<ModelEditorMessage>);
    let mut uploading = use_signal(|| false);
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
    let mut upload_model = move |file: web_sys::File| {
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
            let is_current = {
                let runtime = runtime.clone();
                let owner = owner.clone();
                move || {
                    current_snapshot(
                        &runtime,
                        &owner,
                        &selected,
                        selection_generation(),
                        scope_generation(),
                        &workspace,
                    )
                    .is_some()
                }
            };
            let mut fail = {
                let is_current = is_current.clone();
                let owner = owner.clone();
                move |text: String| {
                    uploading.set(false);
                    if is_current() {
                        error.set(Some(ModelEditorMessage {
                            owner: owner.clone(),
                            text,
                        }));
                    }
                }
            };
            let uploaded = upload_model_asset(
                file,
                &runtime,
                &[],
                is_current.clone(),
                "The selected component changed while importing the model.",
            )
            .await;
            if !alive.get() || request_generation.get() != generation {
                return;
            }
            let (asset, _) = match uploaded {
                Ok(value) => value,
                Err(message) => {
                    fail(message);
                    return;
                }
            };
            let resolver = model_resolver(
                original,
                project_owned_at_start,
                move |definition, document| {
                    let asset_id = if let Some(existing) = document
                        .assets
                        .iter()
                        .find(|existing| existing.sha256 == asset.sha256)
                    {
                        existing.id.clone()
                    } else {
                        let mut asset = asset.clone();
                        // Re-assert uniqueness against the commit-time document.
                        asset.id = crate::parts_custom_definition::unique_suffixed_id_capped(
                            &asset.id,
                            |id| document.assets.iter().any(|existing| existing.id == id),
                        )
                        .unwrap_or_else(|| format!("{}-new", asset.id));
                        let id = asset.id.clone();
                        document.assets.push(asset);
                        id
                    };
                    let mut models = definition.models.clone().unwrap_or_default();
                    let new_model = new_model(asset_id);
                    if models.is_empty() {
                        models.push(new_model);
                    } else {
                        models[0] = new_model;
                    }
                    definition.models = Some(models);
                    Ok(())
                },
            );
            uploading.set(false);
            error.set(None);
            notice.set(None);
            settle_model_commit(&runtime, owner, error, resolver);
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
    // One owner-admitted submit path for transform commits and removal: builds
    // the resolver, then commits inline onto the parent owner-gated error.
    let submit_model_edit = use_callback({
        let runtime = runtime.clone();
        let owner = owner.clone();
        let original = definition.clone();
        let selected = selected;
        let asset_id = binding.as_ref().map(|model| model.asset_id.clone());
        let mut error = error;
        let mut notice = notice;
        move |intent: ModelEditIntent| {
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
            let result = match intent {
                ModelEditIntent::Transform(field, axis, value) => submit_model_transform(
                    live,
                    &original,
                    project_owned,
                    asset_id,
                    field,
                    axis,
                    value,
                ),
                ModelEditIntent::Remove => {
                    submit_remove_model(live, &original, project_owned, asset_id)
                }
            };
            match result {
                Ok(resolver) => {
                    error.set(None);
                    notice.set(None);
                    settle_model_commit(&runtime, owner.clone(), error, resolver);
                }
                Err(message) => error.set(Some(ModelEditorMessage {
                    owner: owner.clone(),
                    text: message,
                })),
            }
        }
    });
    let transform_commit = submit_model_edit;
    let remove_model = move |_| submit_model_edit.call(ModelEditIntent::Remove);
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
        InspectorSection { title: "3D model", default_open: false,
            badge: if binding.is_some() { "Attached" } else { "Optional" },
                if let Some(model) = binding.as_ref() {
                    if let Some(name) = asset_name.as_ref() {
                    p { "Bound asset: {name}" }
                    }
                    fieldset { class: "grid gap-2",
                        legend { "Model alignment" }
                        for (title, unit, positive, vector, field) in [("Offset", "mm", false, model.offset, ModelVector::Offset), ("Rotation", "°", false, model.rotation, ModelVector::Rotation), ("Scale", "×", true, model.scale, ModelVector::Scale)] {
                            ModelTransformFieldset { key: "{owner:?}-{title}", title: format!("{title} · {unit}"), value: vector,
                                min: positive.then_some(0.001),
                                aria_labels: Some([format!("{title} X {unit}"), format!("{title} Y {unit}"), format!("{title} Z {unit}")]),
                                on_commit: move |(axis, number)| {
                                    transform_commit.call(ModelEditIntent::Transform(field, axis, number));
                                },
                            }
                        }
                    }
                    Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, onclick: remove_model, "Remove attached model" }
                } else {
                    p { "No model attached to this component." }
                }
                FilePickerButton {
                    label: "Attach STEP / STL / WRL model",
                    accept: ".step,.stp,.stl,.wrl,model/step,model/stl,model/vrml",
                    disabled: uploading() || current_owner.is_none(),
                    on_files: move |files: Vec<web_sys::File>| if let Some(file) = files.into_iter().next() { upload_model(file); },
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
