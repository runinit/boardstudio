//! Parts-owned retained generator settings drafts and their accepted edit boundary.
use super::editor_fields::{EditorNumberField, EditorSelect, EditorTextField, EditorTextarea};
use super::{GeneratorDraftStore, GeneratorPreviewDraft};
use crate::parts_generator_input::{apply_input, effective_value};
use crate::{presentation::model_asset_import::read_model_file, runtime::Runtime};
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_model::model::{Asset, PartDefinition};
use boardstudio_web_ui_shared::inspector_fields::{CheckboxField, InspectorSection};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonVariant},
    checkbox::CheckboxState,
    field::FieldError,
    spinner::Spinner,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use serde_json::Value;
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

mod resolvers;
pub use resolvers::{generator_apply_resolver, generator_model_upload_resolver};

#[derive(Clone, Debug)]
pub struct GeneratorOwner {
    pub scope: Option<Scope>,
    pub selection: Option<(Option<Scope>, String)>,
    pub session_epoch: boardstudio_application::SessionEpoch,
    pub document_id: String,
    pub definition_id: String,
    pub base_definition: PartDefinition,
    pub source: String,
    pub generator_version: String,
    pub project_owned_at_start: bool,
    pub scope_generation: u64,
    pub selection_generation: u64,
}

impl PartialEq for GeneratorOwner {
    fn eq(&self, other: &Self) -> bool {
        self.scope == other.scope
            && self.selection == other.selection
            && self.session_epoch == other.session_epoch
            && self.document_id == other.document_id
            && self.definition_id == other.definition_id
            && self.source == other.source
            && self.generator_version == other.generator_version
            && self.scope_generation == other.scope_generation
            && self.selection_generation == other.selection_generation
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GeneratorPreviewStatus {
    Pending,
    Ready,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
struct GeneratorParameter {
    key: String,
    label: String,
    kind: String,
    group: &'static str,
    value: Value,
}

#[derive(Clone, Debug, PartialEq)]
struct ScopedFeedback {
    owner: GeneratorOwner,
    message: String,
}

pub fn owner_is_current(
    owner: &GeneratorOwner,
    runtime: &Runtime,
    selected: Signal<Option<(Option<Scope>, String)>>,
    scope_generation: u64,
    selection_generation: u64,
    workspace: &'static str,
) -> bool {
    if !owner_context_is_current(
        owner,
        runtime,
        selected,
        scope_generation,
        selection_generation,
        workspace,
    ) {
        return false;
    }
    let Some(snapshot) = runtime.model().accepted else {
        return false;
    };
    match snapshot
        .document
        .definitions
        .iter()
        .find(|definition| definition.id == owner.definition_id)
    {
        Some(definition) => definition.generator.as_ref().is_some_and(|generator| {
            generator.source == owner.source && generator.version == owner.generator_version
        }),
        None => !owner.project_owned_at_start,
    }
}

fn owner_context_is_current(
    owner: &GeneratorOwner,
    runtime: &Runtime,
    selected: Signal<Option<(Option<Scope>, String)>>,
    scope_generation: u64,
    selection_generation: u64,
    workspace: &'static str,
) -> bool {
    if workspace != "Parts"
        || runtime.scope() != owner.scope
        || selected() != owner.selection
        || scope_generation != owner.scope_generation
        || selection_generation != owner.selection_generation
    {
        return false;
    }
    let Some(snapshot) = runtime.model().accepted else {
        return false;
    };
    if snapshot.session_epoch != owner.session_epoch || snapshot.document.id != owner.document_id {
        return false;
    }
    true
}

fn make_owner(
    snapshot: &AcceptedSnapshot,
    scope: Option<Scope>,
    selection: Option<(Option<Scope>, String)>,
    definition: &PartDefinition,
    scope_generation: u64,
    selection_generation: u64,
) -> Option<GeneratorOwner> {
    let generator = definition.generator.as_ref()?;
    if selection
        .as_ref()
        .is_some_and(|(_, selected_id)| selected_id != &definition.id)
    {
        return None;
    }
    Some(GeneratorOwner {
        scope,
        selection,
        session_epoch: snapshot.session_epoch,
        document_id: snapshot.document.id.clone(),
        definition_id: definition.id.clone(),
        base_definition: definition.clone(),
        source: generator.source.clone(),
        generator_version: generator.version.clone(),
        project_owned_at_start: snapshot
            .document
            .definitions
            .iter()
            .any(|accepted| accepted.id == definition.id),
        scope_generation,
        selection_generation,
    })
}

fn parameters(
    definition: &PartDefinition,
    schema: &BTreeMap<String, Value>,
) -> Vec<GeneratorParameter> {
    let Some(generator) = definition.generator.as_ref() else {
        return Vec::new();
    };
    let mut entries = schema
        .iter()
        .filter_map(|(key, spec)| {
            let kind = spec.get("type")?.as_str()?.to_owned();
            let value = effective_value(None, generator.parameters.get(key), spec)
                .cloned()
                .unwrap_or(Value::Null);
            Some(GeneratorParameter {
                key: key.clone(),
                label: if key == "side" {
                    "Board side".into()
                } else {
                    parameter_label(key)
                },
                group: parameter_group(key, &kind, &definition.kind),
                kind,
                value,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.key.cmp(&right.key));
    entries
}

fn parameter_label(key: &str) -> String {
    key.replace('_', " ")
        .split_whitespace()
        .map(|word| match word.to_ascii_lowercase().as_str() {
            "pcb" => "PCB".to_owned(),
            "xyz" => "XYZ".to_owned(),
            "led" => "LED".to_owned(),
            "rgb" => "RGB".to_owned(),
            _ => {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().chain(chars).collect())
                    .unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parameter_group(
    key: &str,
    kind: &str,
    part_kind: &boardstudio_model::model::PartKind,
) -> &'static str {
    if key.contains("3dmodel") || key.starts_with("model_") {
        "3D model placement"
    } else if kind == "net" {
        "Connections"
    } else if matches!(key, "keycap_width" | "keycap_height" | "keycap_depth") {
        "Keycap dimensions"
    } else if matches!(
        key,
        "side"
            | "reversible"
            | "hotswap"
            | "solder"
            | "include_keycap"
            | "choc_v1_support"
            | "choc_v2_support"
            | "name"
            | "text"
    ) || (matches!(part_kind, boardstudio_model::model::PartKind::Utility)
        && !key.contains("pad")
        && !key.contains("trace")
        && !key.contains("drill")
        && matches!(kind, "number" | "string" | "boolean"))
    {
        "Footprint options"
    } else {
        "Advanced footprint options"
    }
}

fn input_value(value: &Value, kind: &str) -> String {
    match (value, kind) {
        (Value::String(value), _) => value.clone(),
        (Value::Null, _) => String::new(),
        (Value::Array(_) | Value::Object(_), _) => {
            serde_json::to_string_pretty(value).unwrap_or_default()
        }
        _ => value.to_string(),
    }
}

pub async fn prepare_generator_candidate(
    definition: PartDefinition,
    schema: BTreeMap<String, Value>,
    edits: BTreeMap<String, Value>,
) -> Result<PartDefinition, String> {
    let candidate = apply_input(&definition, &schema, &edits)?;
    let normalized = super::catalogue::normalize_generator_definition(candidate).await?;
    let Some(_drawings) =
        crate::presentation::footprint_graphics::generator_drawings(normalized.clone(), None, None)
            .await
            .map_err(|error| format!("Generator preview failed: {error}"))?
    else {
        return Err("The retained generator did not produce a preview for this definition.".into());
    };
    Ok(normalized)
}

/// Dynamic settings form. Each field remains a local JSON draft until Apply; every
/// candidate preview uses the same retained generator renderer as the Parts canvas.
#[component]
pub fn GeneratorSettingsEditor(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    selected: Signal<Option<(Option<Scope>, String)>>,
    definition: PartDefinition,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let store = use_context::<GeneratorDraftStore>().0;
    let scope_generation = use_context::<super::super::SelectionAdapter>().generation;
    let selection_generation = use_context::<super::PartsSelectionGeneration>().0;
    let workspace = use_context::<super::super::WorkspaceState>().0;
    let _ = use_context::<Signal<u64>>()();
    let generator_source = definition
        .generator
        .as_ref()
        .map(|generator| generator.source.clone())
        .unwrap_or_default();
    let schema = use_resource(use_reactive((&generator_source,), |(source,)| async move {
        super::catalogue::generator_parameter_schema(&source).await
    }));
    let schema_result = schema.read().clone();
    let edits = use_signal(BTreeMap::<String, Value>::new);
    let mut changed_parameters = use_signal(BTreeSet::<String>::new);
    let mut feedback = use_signal(|| None::<ScopedFeedback>);

    let mut uploaded_preview_generation = use_signal(|| 0_u64);
    let sequence = use_hook(|| Rc::new(Cell::new(0_u64)));
    let model_sequence = use_hook(|| Rc::new(Cell::new(0_u64)));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let mut model_uploading = use_signal(|| false);
    use_drop({
        let alive = alive.clone();
        let sequence = sequence.clone();
        let model_sequence = model_sequence.clone();
        move || {
            alive.set(false);
            sequence.set(sequence.get().wrapping_add(1));
            model_sequence.set(model_sequence.get().wrapping_add(1));
        }
    });
    let owner = make_owner(
        &snapshot,
        scope.clone(),
        selected(),
        &definition,
        scope_generation(),
        selection_generation(),
    );
    use_effect(use_reactive((&owner,), {
        let mut edits = edits;
        let mut store = store;
        move |_| {
            edits.set(BTreeMap::new());
            changed_parameters.set(BTreeSet::new());
            store.set(None);
        }
    }));

    let Some(owner) = owner else {
        return rsx! {};
    };
    let schema = match schema_result {
        Some(Ok(schema)) => schema,
        Some(Err(error)) => {
            return rsx! { Alert { variant: AlertVariant::Destructive, "Generator settings could not be loaded: {error}" } };
        }
        None => {
            return rsx! { div { role: "status", class: "flex items-center gap-2", Spinner {} "Loading generator settings…" } };
        }
    };
    let entries = parameters(&definition, &schema);
    let initial_parameters = schema
        .iter()
        .filter_map(|(key, spec)| {
            let saved = definition
                .generator
                .as_ref()
                .and_then(|generator| generator.parameters.get(key));
            effective_value(None, saved, spec).map(|value| (key.clone(), value.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut changed = edits;
    let callback_owner = owner.clone();
    let callback_runtime = runtime.clone();
    let callback_schema = schema.clone();
    let preview_alive = alive.clone();
    let preview_sequence = sequence.clone();
    let request_preview = use_callback(move |next: BTreeMap<String, Value>| {
        changed.set(next.clone());
        feedback.set(None);
        let request_owner = callback_owner.clone();
        let mut preview_store = store;
        preview_store.set(Some(GeneratorPreviewDraft {
            owner: request_owner.clone(),
            definition: None,
            status: GeneratorPreviewStatus::Pending,
        }));
        let base = definition.clone();
        let schema = callback_schema.clone();
        let scope_generation = scope_generation;
        let selection_generation = selection_generation;
        let selected = selected;
        let workspace = workspace;
        let runtime = callback_runtime.clone();
        let mut store = store;
        let task = preview_sequence.clone();
        let alive = preview_alive.clone();
        let ticket = task.get().wrapping_add(1);
        task.set(ticket);
        spawn_local(async move {
            let result = prepare_generator_candidate(base, schema, next).await;
            if !alive.get()
                || task.get() != ticket
                || !owner_is_current(
                    &request_owner,
                    &runtime,
                    selected,
                    scope_generation(),
                    selection_generation(),
                    workspace(),
                )
            {
                return;
            }
            store.set(Some(match result {
                Ok(candidate) => GeneratorPreviewDraft {
                    owner: request_owner,
                    definition: Some(candidate),
                    status: GeneratorPreviewStatus::Ready,
                },
                Err(error) => GeneratorPreviewDraft {
                    owner: request_owner,
                    definition: None,
                    status: GeneratorPreviewStatus::Failed(error),
                },
            }));
        });
    });
    let change_parameter = use_callback(move |(key, value): (String, Value)| {
        let mut next = edits();
        changed_parameters.write().insert(key.clone());
        next.insert(key, value);
        request_preview.call(next);
    });
    // The form and its first transient preview must use the same saved/default values.
    // Owner changes also retire the previous draft and start a fresh guarded request.
    use_effect(use_reactive((&owner, &schema), move |_| {
        request_preview.call(initial_parameters.clone());
    }));
    use_effect(use_reactive(
        (&uploaded_preview_generation(),),
        move |(generation,)| {
            if generation > 0 {
                let remaining = edits.peek().clone();
                request_preview.call(remaining);
            }
        },
    ));
    let active_draft = store().filter(|draft| draft.owner == owner);
    let preview_ready = active_draft.as_ref().is_some_and(|draft| {
        draft.status == GeneratorPreviewStatus::Ready && draft.definition.is_some()
    });
    let model_import_runtime = runtime.clone();
    let model_import_owner = owner.clone();
    let import_sequence = sequence.clone();
    let import_generator_model = use_callback(move |(parameter, file): (String, web_sys::File)| {
        let runtime = model_import_runtime.clone();
        let owner = model_import_owner.clone();
        if model_uploading() || !parameter.ends_with("3dmodel_filename") {
            return;
        }
        if !owner_is_current(
            &owner,
            &runtime,
            selected,
            scope_generation(),
            selection_generation(),
            workspace(),
        ) {
            return;
        }
        feedback.set(None);
        model_uploading.set(true);
        let ticket = model_sequence.get().wrapping_add(1);
        model_sequence.set(ticket);
        let model_sequence = model_sequence.clone();
        let alive = alive.clone();
        let runtime = runtime.clone();
        let owner = owner.clone();
        let error_prefix = parameter_label(&parameter).to_ascii_lowercase();
        let mut feedback = feedback;
        let mut model_uploading = model_uploading;

        let sequence = import_sequence.clone();
        let mut edits = edits;
        let mut store = store;
        let mut changed_parameters = changed_parameters;
        spawn_local(async move {
            let imported = read_model_file(file).await;
            if !alive.get() || model_sequence.get() != ticket {
                return;
            }
            let imported = match imported {
                Ok(imported) => imported,
                Err(message) => {
                    model_uploading.set(false);
                    if owner_context_is_current(
                        &owner,
                        &runtime,
                        selected,
                        scope_generation(),
                        selection_generation(),
                        workspace(),
                    ) {
                        feedback.set(Some(ScopedFeedback {
                            owner,
                            message: format!("{error_prefix}: {message}"),
                        }));
                    }
                    return;
                }
            };
            if !owner_is_current(
                &owner,
                &runtime,
                selected,
                scope_generation(),
                selection_generation(),
                workspace(),
            ) {
                model_uploading.set(false);
                if owner_context_is_current(
                    &owner,
                    &runtime,
                    selected,
                    scope_generation(),
                    selection_generation(),
                    workspace(),
                ) {
                    feedback.set(Some(ScopedFeedback {
                        owner,
                        message: format!("{error_prefix}: The selected generator changed while the model was being read. Re-select it before importing another model."),
                    }));
                }
                return;
            }
            if let Err(message) = imported.store(&runtime.store).await {
                model_uploading.set(false);
                if owner_context_is_current(
                    &owner,
                    &runtime,
                    selected,
                    scope_generation(),
                    selection_generation(),
                    workspace(),
                ) {
                    feedback.set(Some(ScopedFeedback {
                        owner,
                        message: format!("{error_prefix}: {message}"),
                    }));
                }
                return;
            }
            if !alive.get() || model_sequence.get() != ticket {
                return;
            }
            if !owner_is_current(
                &owner,
                &runtime,
                selected,
                scope_generation(),
                selection_generation(),
                workspace(),
            ) {
                model_uploading.set(false);
                if owner_context_is_current(
                    &owner,
                    &runtime,
                    selected,
                    scope_generation(),
                    selection_generation(),
                    workspace(),
                ) {
                    feedback.set(Some(ScopedFeedback {
                        owner,
                        message: format!("{error_prefix}: The selected generator changed before the model could be attached."),
                    }));
                }
                return;
            }
            let Some(current) = runtime.model().accepted.clone() else {
                model_uploading.set(false);
                feedback.set(Some(ScopedFeedback {
                    owner,
                    message: format!("{error_prefix}: The accepted project is unavailable."),
                }));
                return;
            };
            let latest_definition = current
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == owner.definition_id)
                .cloned()
                .unwrap_or_else(|| owner.base_definition.clone());
            if latest_definition
                .generator
                .as_ref()
                .is_none_or(|generator| {
                    generator.source != owner.source || generator.version != owner.generator_version
                })
            {
                model_uploading.set(false);
                feedback.set(Some(ScopedFeedback {
                    owner,
                    message: format!("{error_prefix}: The selected generator settings changed while the model was being saved."),
                }));
                return;
            }
            // The bytes are already stored by hash; the resolver chooses the asset
            // identity against the accepted assets and adds only the Asset metadata.
            let asset = Asset {
                id: String::new(),
                name: imported.filename.clone(),
                media_type: imported.media_type.clone(),
                sha256: imported.sha256.clone(),
                license: None,
                source: Some("local file".into()),
            };
            model_uploading.set(false);
            let asset_seed = runtime.operation().0;
            let result = runtime.commit(
                "parts-generator-settings",
                generator_model_upload_resolver(
                    owner.clone(),
                    latest_definition,
                    parameter.clone(),
                    asset,
                    asset_seed,
                ),
            );
            match result {
                Ok(_) => {
                    edits.write().remove(&parameter);
                    changed_parameters.write().remove(&parameter);
                    sequence.set(sequence.get().wrapping_add(1));
                    store.set(None);
                    uploaded_preview_generation += 1;
                    feedback.set(None);
                }
                Err(message) => feedback.set(Some(ScopedFeedback { owner, message })),
            }
        });
    });
    let on_apply = {
        let runtime = runtime.clone();

        let sequence = sequence.clone();
        let mut store = store;
        let mut edits = edits;
        let mut feedback = feedback;
        let current_owner = active_draft
            .as_ref()
            .map(|draft| draft.owner.clone())
            .unwrap_or_else(|| owner.clone());
        let selected = selected;
        let active_draft = active_draft.clone();
        move |_| {
            let Some(candidate) = active_draft
                .as_ref()
                .filter(|draft| draft.status == GeneratorPreviewStatus::Ready)
                .and_then(|draft| draft.definition.clone())
            else {
                return;
            };
            if !owner_context_is_current(
                &current_owner,
                &runtime,
                selected,
                scope_generation(),
                selection_generation(),
                workspace(),
            ) {
                return;
            }
            let result = runtime.commit(
                "parts-generator-settings",
                generator_apply_resolver(current_owner.clone(), candidate, changed_parameters()),
            );
            sequence.set(sequence.get().wrapping_add(1));
            store.set(None);
            edits.set(BTreeMap::new());
            changed_parameters.set(BTreeSet::new());
            feedback.set(result.err().map(|message| ScopedFeedback {
                owner: current_owner.clone(),
                message,
            }));
        }
    };

    let groups = [
        "Footprint options",
        "Keycap dimensions",
        "Connections",
        "3D model placement",
        "Advanced footprint options",
    ];
    let mut open_groups = use_signal(|| {
        BTreeSet::from([
            "Footprint options".to_owned(),
            "Keycap dimensions".to_owned(),
        ])
    });
    rsx! {
        section { class: "grid gap-2", "aria-label": "Generator settings",
            for group in groups {
                { let group_entries = entries.iter().filter(|entry| entry.group == group).cloned().collect::<Vec<_>>();
                  let group_name = group.to_owned();
                  let is_open = open_groups().contains(group);
                  rsx! {
                    if !group_entries.is_empty() {
                    InspectorSection {
                        title: "{group}",
                        open: is_open,
                        on_open_change: move |next: bool| {
                            if next {
                                open_groups.write().insert(group_name.clone());
                            } else {
                                open_groups.write().remove(&group_name);
                            }
                        },
                        fieldset {
                            class: "grid gap-2",
                            "aria-label": "{group}",
                            for entry in group_entries {
                                GeneratorParameterField {
                                    entry: entry.clone(),
                                    value: edits().get(&entry.key).cloned().unwrap_or(entry.value.clone()),
                                    on_change: EventHandler::new({ let key = entry.key.clone(); move |value| change_parameter.call((key.clone(), value)) }),
                                    on_import: import_generator_model,
                                    busy: model_uploading(),
                                    feedback: feedback()
                                        .filter(|message| owner_context_is_current(
                                            &message.owner,
                                            &runtime,
                                            selected,
                                            scope_generation(),
                                            selection_generation(),
                                            workspace(),
                                        ))
                                        .map(|message| message.message),
                                }
                            }
                        }
                    }
                    }
                  }
                }
            }
            if let Some(draft) = active_draft.as_ref() {
                match &draft.status {
                    GeneratorPreviewStatus::Pending => rsx! { div { role: "status", class: "flex items-center gap-2", Spinner {} "Generating footprint preview…" } },
                    GeneratorPreviewStatus::Ready => rsx! { div { role: "status", "Current generator preview is ready to apply." } },
                    GeneratorPreviewStatus::Failed(error) => rsx! { Alert { variant: AlertVariant::Destructive, "Generator preview failed: {error}" } },
                }
            }
            if model_uploading() { div { role: "status", class: "flex items-center gap-2", Spinner {} "Reading and saving generator model…" } }
            if let Some(message) = feedback()
                .filter(|message| owner_context_is_current(
                    &message.owner,
                    &runtime,
                    selected,
                    scope_generation(),
                    selection_generation(),
                    workspace(),
                ))
            {
                Alert { variant: AlertVariant::Destructive, "{message.message}" }
            }
            Button {
                variant: ButtonVariant::Default,
                class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                disabled: !preview_ready,
                onclick: on_apply,
                "Apply generator settings"
            }
        }
    }
}

#[component]
fn GeneratorParameterField(
    entry: GeneratorParameter,
    value: Value,
    on_change: EventHandler<Value>,
    on_import: EventHandler<(String, web_sys::File)>,
    busy: bool,
    feedback: Option<String>,
) -> Element {
    let key = entry.key.clone();
    let label = entry.label.clone();
    let mut file_input = use_signal(|| None::<HtmlInputElement>);
    if key.to_ascii_lowercase().ends_with("3dmodel_filename") {
        let current = input_value(&value, &entry.kind);
        let field_error = feedback.filter(|message| {
            message
                .to_ascii_lowercase()
                .starts_with(&key.replace('_', " ").to_ascii_lowercase())
        });
        return rsx! {
            div { class: "grid gap-2",
                EditorTextField { label: "{label}", aria_label: "{key}", value: "{current}", readonly: true }
                Button {
                    variant: ButtonVariant::Secondary,
                    class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: busy,
                    onclick: move |_| {
                        if let Some(input) = file_input() {
                            input.click();
                        }
                    },
                    "Attach STEP / STL / WRL"
                }
                input {
                    r#type: "file",
                    hidden: true,
                    accept: ".step,.stp,.stl,.wrl,model/step,model/stl,model/vrml",
                    aria_label: "Attach STEP / STL / WRL",
                    disabled: busy,
                    onmounted: move |event: MountedEvent| {
                        file_input.set(event.data().downcast::<web_sys::Element>().and_then(|e| e.dyn_ref::<HtmlInputElement>()).cloned());
                    },
                    onchange: move |event| {
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
                        on_import.call((key.clone(), file));
                    }
                }
                if !current.is_empty() {
                    Button {
                        variant: ButtonVariant::Default,
                        class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: busy,
                        onclick: move |_| on_change.call(Value::String(String::new())),
                        "Remove model"
                    }
                }
                if let Some(message) = field_error { FieldError { "{message}" } }
            }
        };
    }
    if key == "side" && matches!(value.as_str(), Some("F" | "B")) {
        let selected = value.as_str().unwrap_or("F").to_owned();
        return rsx! {
            EditorSelect {
                label: label.clone(),
                aria_label: "side",
                value: selected,
                options: vec![("F".to_owned(), "Front".to_owned()), ("B".to_owned(), "Back".to_owned())],
                on_select: move |value: String| on_change.call(Value::String(value)),
            }
        };
    }
    match entry.kind.as_str() {
        "boolean" => rsx! {
            CheckboxField {
                id: format!("generator-toggle-{key}-{}", dioxus::core::current_scope_id().0),
                label: "{label}",
                aria_label: Some(key.clone()),
                checked: if value.as_bool().unwrap_or(false) { CheckboxState::Checked } else { CheckboxState::Unchecked },
                on_checked_change: move |state: CheckboxState| on_change.call(Value::Bool(bool::from(state))),
            }
        },
        "array" | "object" | "anchor" => {
            let current = input_value(&value, &entry.kind);
            rsx! {
                EditorTextarea {
                    label: "{label}",
                    aria_label: "{key}",
                    rows: 3,
                    spellcheck: false,
                    value: "{current}",
                    oninput: move |event: FormEvent| on_change.call(Value::String(event.value())),
                }
            }
        }
        "number" => {
            let current = input_value(&value, "number");
            rsx! {
                EditorNumberField {
                    label: "{label}",
                    aria_label: "{label}",
                    step: "any",
                    value: "{current}",
                    oninput: move |event: FormEvent| on_change.call(Value::String(event.value())),
                }
            }
        }
        _ => {
            let current = input_value(&value, &entry.kind);
            rsx! {
                EditorTextField {
                    label: "{label}",
                    aria_label: "{key}",
                    value: "{current}",
                    oninput: move |event: FormEvent| on_change.call(Value::String(event.value())),
                }
            }
        }
    }
}
