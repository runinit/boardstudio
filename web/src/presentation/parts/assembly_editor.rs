//! Parts-owned saved assembly list and the first authoring slice of F4.6.
use boardstudio_application::{AcceptedSnapshot, Event, Scope, TerminalOutcome};
use boardstudio_core::model::{
    AssemblyDefinition, AssemblyMember, Asset, EditCommand, EditOperation, EditPhase,
    PartDefinition, PartKind, PartModel, Pose2, ProjectDoc, Side, Vec2, Vec3,
};
use dioxus::prelude::*;
use js_sys::{Date, Function, Reflect};
use std::{collections::BTreeSet, rc::Rc};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

#[derive(Clone, Debug, PartialEq)]
struct AssemblyDraft {
    base: Option<AssemblyDefinition>,
    value: AssemblyDefinition,
    assets: Vec<Asset>,
    document_id: String,
    session_epoch: boardstudio_application::SessionEpoch,
    scope: Option<Scope>,
}

#[derive(Clone)]
struct PendingSave {
    assembly_id: String,
    outcome: crate::operation_outcomes::OutcomeSlot,
}

/// Saved reusable assemblies are project data; editor fields stay local until one
/// accepted document edit commits them through the existing Session path.
#[component]
pub(super) fn SavedAssembliesEditor(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    definitions: Vec<PartDefinition>,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let workspace = use_context::<super::super::WorkspaceState>().0;
    let runtime_version = use_context::<Signal<u64>>();
    let _ = runtime_version();
    let mut editing = use_signal(|| None::<AssemblyDraft>);
    let mut pending = use_signal(|| None::<PendingSave>);
    let mut feedback = use_signal(|| None::<String>);

    use_effect({
        let runtime = runtime.clone();
        move || {
            let _ = runtime_version();
            let Some(waiting) = pending.read().clone() else {
                return;
            };
            let Some(outcome) = waiting.outcome.borrow().clone() else {
                return;
            };
            pending.set(None);
            match outcome {
                TerminalOutcome::Completed => {
                    let accepted = runtime.model().accepted;
                    if let Some(snapshot) = accepted {
                        if let Some(saved) = snapshot
                            .document
                            .assemblies
                            .iter()
                            .find(|assembly| assembly.id == waiting.assembly_id)
                            .cloned()
                        {
                            editing.with_mut(|draft| {
                                if let Some(draft) = draft.as_mut() {
                                    if draft.value.id == saved.id {
                                        draft.base = Some(saved.clone());
                                        draft.value = saved;
                                        draft.assets.clear();
                                    }
                                }
                            });
                            feedback.set(Some(
                                "Assembly saved. Existing placements are unchanged.".into(),
                            ));
                        }
                    }
                }
                TerminalOutcome::Rejected(reason) => feedback.set(Some(reason)),
                TerminalOutcome::PersistenceFailed(reason) => feedback.set(Some(format!(
                    "Assembly edit was accepted, but saving failed: {reason}"
                ))),
                TerminalOutcome::Superseded => feedback.set(Some(
                    "The assembly edit was superseded by a newer project operation.".into(),
                )),
                TerminalOutcome::Cancelled => {
                    feedback.set(Some("The assembly edit was cancelled.".into()))
                }
                TerminalOutcome::BlockedByRecovery(reason) => feedback.set(Some(reason)),
                TerminalOutcome::ExecutorFailed(reason) => feedback.set(Some(reason)),
                TerminalOutcome::Closed => feedback.set(Some(
                    "The project session closed before the assembly edit completed.".into(),
                )),
            }
        }
    });

    let open_existing = {
        let snapshot = snapshot.clone();
        let scope = scope.clone();
        move |assembly: AssemblyDefinition| {
            feedback.set(None);
            editing.set(Some(AssemblyDraft {
                base: Some(assembly.clone()),
                value: assembly,
                assets: Vec::new(),
                document_id: snapshot.document.id.clone(),
                session_epoch: snapshot.session_epoch,
                scope: scope.clone(),
            }));
        }
    };
    let new_assembly = {
        let snapshot = snapshot.clone();
        let scope = scope.clone();
        move |_| match next_id(&snapshot.document) {
            Ok(id) => {
                feedback.set(None);
                editing.set(Some(AssemblyDraft {
                    base: None,
                    value: AssemblyDefinition {
                        id,
                        name: "New assembly".into(),
                        members: Vec::new(),
                    },
                    assets: Vec::new(),
                    document_id: snapshot.document.id.clone(),
                    session_epoch: snapshot.session_epoch,
                    scope: scope.clone(),
                }));
            }
            Err(error) => feedback.set(Some(error)),
        }
    };

    let duplicate = {
        let snapshot = snapshot.clone();
        let scope = scope.clone();
        move |assembly: AssemblyDefinition| match next_id(&snapshot.document) {
            Ok(id) => {
                let value = AssemblyDefinition {
                    id,
                    name: format!("{} copy", assembly.name),
                    members: assembly.members.clone(),
                };
                feedback.set(None);
                editing.set(Some(AssemblyDraft {
                    base: None,
                    value,
                    assets: Vec::new(),
                    document_id: snapshot.document.id.clone(),
                    session_epoch: snapshot.session_epoch,
                    scope: scope.clone(),
                }));
            }
            Err(error) => feedback.set(Some(error)),
        }
    };

    let save = {
        let runtime = runtime.clone();
        let scope = scope.clone();
        let definitions = definitions.clone();
        move |_| {
            if pending.read().is_some() || workspace() != "Parts" {
                return;
            }
            let Some(draft) = editing.read().clone() else {
                return;
            };
            let current_scope = runtime.scope();
            let Some(current) = runtime.model().accepted else {
                feedback.set(Some("The accepted project is unavailable.".into()));
                return;
            };
            if scope != current_scope
                || draft.scope != current_scope
                || draft.document_id != current.document.id
                || draft.session_epoch != current.session_epoch
            {
                feedback.set(Some("The project or Parts scope changed while this assembly draft was open. Reopen it before saving.".into()));
                return;
            }
            let candidate = match saved_document(&current.document, &draft, &definitions) {
                Ok(document) => document,
                Err(error) => {
                    feedback.set(Some(error));
                    return;
                }
            };
            let operation_id = runtime.operation();
            let event = Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: current.document.revision,
                    transaction_id: format!("parts-assembly-editor-{}", operation_id.0),
                    phase: EditPhase::Commit,
                    target_ids: vec![draft.value.id.clone()],
                    operation: EditOperation::ReplaceDocument {
                        document: Box::new(candidate),
                    },
                },
            };
            let outcome = runtime.observe_operation(operation_id);
            pending.set(Some(PendingSave {
                assembly_id: draft.value.id,
                outcome,
            }));
            feedback.set(None);
            runtime.submit(event);
        }
    };

    let saved_assemblies = snapshot.document.assemblies.clone();
    let draft = editing.read().clone();
    let draft_key = draft
        .as_ref()
        .map(|draft| draft.value.id.clone())
        .unwrap_or_default();
    rsx! {
        section { class: "m1-parts-assemblies", "aria-label": "Saved assemblies",
            h2 { "Assemblies" }
            button { class: "m1-parts-create-component", r#type: "button", disabled: pending.read().is_some(), onclick: new_assembly, "New assembly" }
            if saved_assemblies.is_empty() {
                p { class: "m1-parts-empty", "No saved assemblies yet." }
            } else {
                div { class: "m1-parts-assembly-saved-list", role: "list", "aria-label": "Saved assemblies",
                    for assembly in saved_assemblies {
                        { let mut open_existing = open_existing.clone(); let mut duplicate = duplicate.clone();
                          let existing_assembly = assembly.clone(); let duplicate_assembly = assembly.clone();
                          rsx! {
                            div { class: "m1-parts-assembly-saved-row", key: "{assembly.id}",
                                button { r#type: "button", disabled: pending.read().is_some(), onclick: move |_| open_existing(existing_assembly.clone()), "{assembly.name}" }
                                button { r#type: "button", disabled: pending.read().is_some(), aria_label: "Duplicate {assembly.name}", onclick: move |_| duplicate(duplicate_assembly.clone()), "Duplicate" }
                            }
                          }
                        }
                    }
                }
            }
            if let Some(message) = feedback.read().as_ref() {
                p { class: "m1-parts-assembly-feedback", role: "status", "{message}" }
            }
            if let Some(draft) = draft {
                AssemblyDraftFields {
                    key: "{draft_key}",
                    snapshot: snapshot.clone(),
                    draft,
                    definitions: definitions.clone(),
                    pending: pending.read().is_some(),
                    on_change: move |updated: AssemblyDraft| { editing.set(Some(updated)); feedback.set(None); },
                    on_save: save,
                    on_close: move |_| { editing.set(None); feedback.set(None); },
                }
            }
        }
    }
}

#[component]
fn AssemblyDraftFields(
    snapshot: AcceptedSnapshot,
    draft: AssemblyDraft,
    definitions: Vec<PartDefinition>,
    pending: bool,
    on_change: EventHandler<AssemblyDraft>,
    on_save: EventHandler<MouseEvent>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    let value = use_signal(|| draft.value.clone());
    let assets = use_signal(|| draft.assets.clone());
    let mut import_pending = use_signal(|| false);
    let render_value = value();
    let member_definitions = definitions
        .iter()
        .filter(|definition| {
            !matches!(&definition.kind, boardstudio_core::model::PartKind::Utility)
        })
        .cloned()
        .collect::<Vec<_>>();
    let recipe = assembly_preview_recipe(&render_value, &member_definitions, &assets());
    let preview_definition = recipe
        .first()
        .map(|member| Rc::new(member.definition.clone()));
    let preview_identity = serde_json::to_string(&render_value).unwrap_or_default();
    let controls_disabled = pending || import_pending();
    rsx! {
        section { class: "m1-parts-assembly-editor", "aria-label": "Assembly editor",
            h3 { "Assembly editor" }
            label { class: "m1-parts-assembly-field", "Name"
                { let name_draft = draft.clone(); let name_change = on_change.clone();
                rsx! { input { value: "{render_value.name}", disabled: controls_disabled, oninput: move |event| { let mut value = value; value.with_mut(|value| value.name = event.value()); publish_draft(value, assets, &name_draft, name_change.clone()); } } }
                }
            }
            for member in render_value.members.clone() {
                AssemblyMemberFields {
                    key: "{member.id}",
                    member,
                    draft: draft.clone(),
                    snapshot: snapshot.clone(),
                    definitions: member_definitions.clone(),
                    value,
                    assets,
                    disabled: controls_disabled,
                    import_pending,
                    on_change: on_change.clone(),
                }
            }
            { let add_draft = draft.clone(); let add_change = on_change.clone();
              let mut add_value = value; let add_assets = assets;
              rsx! { button { r#type: "button", disabled: controls_disabled, onclick: move |_| {
                  if let Some(id) = member_id(&add_value()) {
                      add_value.with_mut(|value| value.members.push(AssemblyMember {
                          parameters: None,
                          model_mode: Some(boardstudio_core::model::AssemblyModelMode::Defaults),
                          id,
                          definition_id: None,
                          pose: Pose2 { at: Vec2 { x: 0.0, y: 0.0 }, rotation: 0.0 },
                          side: Side::Front,
                          models: Vec::new(),
                      }));
                      publish_draft(add_value, add_assets, &add_draft, add_change.clone());
                  }
              }, "Add component" } }
            }
            div { class: "m1-parts-assembly-actions",
                button { r#type: "button", disabled: controls_disabled, onclick: on_save, "Save assembly" }
                button { r#type: "button", disabled: controls_disabled, onclick: on_close, "Close editor" }
            }
            if !recipe.is_empty() {
                div { class: "m1-parts-assembly-preview",
                    super::PartsPreviewPanel {
                        definition: preview_definition,
                        recipe,
                        recipe_error: None,
                        recipe_pending: false,
                        recipe_identity: format!("assembly-draft:{}", preview_identity),
                        preview_title: Some(format!("{} preview", render_value.name)),
                        scope: draft.scope.clone(),
                        snapshot_token: snapshot.token,
                        generator_draft: None,
                        start_in_3d: true,
                    }
                }
            } else {
                p { class: "m1-parts-empty", role: "status", "Add a component or model to preview this assembly." }
            }
        }
    }
}

#[component]
fn AssemblyMemberFields(
    member: AssemblyMember,
    draft: AssemblyDraft,
    snapshot: AcceptedSnapshot,
    definitions: Vec<PartDefinition>,
    value: Signal<AssemblyDefinition>,
    assets: Signal<Vec<Asset>>,
    disabled: bool,
    import_pending: Signal<bool>,
    on_change: EventHandler<AssemblyDraft>,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let workspace = use_context::<super::super::WorkspaceState>().0;
    let mut import_feedback = use_signal(|| None::<String>);
    let definition = member
        .definition_id
        .as_deref()
        .and_then(|id| definitions.iter().find(|definition| definition.id == id))
        .cloned();
    let options = model_options(&snapshot.document.assets, &assets());
    let is_custom = matches!(
        &member.model_mode,
        Some(boardstudio_core::model::AssemblyModelMode::Custom)
    ) || (member.model_mode.is_none() && !member.models.is_empty());
    let definition_models = definition
        .as_ref()
        .and_then(|definition| definition.models.clone())
        .unwrap_or_default();
    let model_defaults = definition_models.clone();
    let current_models = member.models.clone();
    let component_id = member.id.clone();
    let definition_id = member.id.clone();
    let side_id = member.id.clone();
    let x_id = member.id.clone();
    let y_id = member.id.clone();
    let rotation_id = member.id.clone();
    let mode_id = member.id.clone();
    let remove_id = member.id.clone();
    let edit_defaults_id = member.id.clone();
    let add_model_id = member.id.clone();
    let definition_draft = draft.clone();
    let side_draft = draft.clone();
    let x_draft = draft.clone();
    let y_draft = draft.clone();
    let rotation_draft = draft.clone();
    let mode_draft = draft.clone();
    let remove_draft = draft.clone();
    let definition_change = on_change.clone();
    let side_change = on_change.clone();
    let x_change = on_change.clone();
    let y_change = on_change.clone();
    let rotation_change = on_change.clone();
    let mode_change = on_change.clone();
    let remove_change = on_change.clone();
    let component_value = value;

    let mounted = use_hook(|| Rc::new(std::cell::Cell::new(true)));
    use_drop({
        let mounted = mounted.clone();
        move || mounted.set(false)
    });
    let import_model = {
        let runtime = runtime.clone();
        let mounted = mounted.clone();
        let draft = draft.clone();
        let member_id = member.id.clone();
        let value = value;
        let assets = assets;
        let mut import_pending = import_pending;
        let mut import_feedback = import_feedback;
        let on_change = on_change.clone();
        move |event: FormEvent| {
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
            if disabled || import_pending() {
                return;
            }
            import_pending.set(true);
            import_feedback.set(None);
            let runtime = runtime.clone();
            let mounted = mounted.clone();
            let draft = draft.clone();
            let member_id = member_id.clone();
            let value = value;
            let assets = assets;
            let mut import_pending = import_pending;
            let mut import_feedback = import_feedback;
            let on_change = on_change.clone();
            spawn_local(async move {
                if !assembly_owner_current(&runtime, &draft, workspace()) {
                    import_feedback.set(Some("The project or Parts scope changed. Reopen the assembly before importing a model.".into()));
                    import_pending.set(false);
                    return;
                }
                let imported =
                    match crate::presentation::model_asset_import::read_model_file(file).await {
                        Ok(imported) => imported,
                        Err(error) => {
                            if mounted.get() {
                                import_feedback.set(Some(error));
                            }
                            import_pending.set(false);
                            return;
                        }
                    };
                if !mounted.get() || !assembly_owner_current(&runtime, &draft, workspace()) {
                    import_feedback.set(Some(
                        "The project or Parts scope changed while reading the model.".into(),
                    ));
                    import_pending.set(false);
                    return;
                }
                if let Err(error) = imported.store(&runtime.store).await {
                    if mounted.get() {
                        import_feedback.set(Some(error));
                    }
                    import_pending.set(false);
                    return;
                }
                if !mounted.get() || !assembly_owner_current(&runtime, &draft, workspace()) {
                    import_feedback.set(Some(
                        "The project or Parts scope changed while storing the model.".into(),
                    ));
                    import_pending.set(false);
                    return;
                }
                let current = runtime.model().accepted;
                let Some(current) = current else {
                    import_feedback.set(Some("The accepted project is unavailable.".into()));
                    import_pending.set(false);
                    return;
                };
                let asset = current
                    .document
                    .assets
                    .iter()
                    .chain(assets().iter())
                    .find(|asset| asset.sha256 == imported.sha256)
                    .cloned()
                    .unwrap_or_else(|| {
                        let operation = runtime.operation();
                        let root = format!("model-asset-{}", operation.0);
                        let id = unique_asset_id(&root, &current.document.assets, &assets());
                        Asset {
                            id,
                            name: imported.filename.clone(),
                            media_type: imported.media_type.clone(),
                            sha256: imported.sha256.clone(),
                            license: None,
                            source: Some("local file".into()),
                        }
                    });
                if !current
                    .document
                    .assets
                    .iter()
                    .any(|existing| existing.id == asset.id)
                    && !assets().iter().any(|existing| existing.id == asset.id)
                {
                    assets.with_mut(|assets| assets.push(asset.clone()));
                }
                component_value.with_mut(|assembly| {
                    if let Some(member) = assembly
                        .members
                        .iter_mut()
                        .find(|member| member.id == member_id)
                    {
                        member.model_mode =
                            Some(boardstudio_core::model::AssemblyModelMode::Custom);
                        member.models.push(default_model(asset.id.clone()));
                    }
                });
                publish_draft(component_value, assets, &draft, on_change);
                import_feedback.set(None);
                import_pending.set(false);
            });
        }
    };

    rsx! {
        fieldset { class: "m1-parts-assembly-member", key: "{component_id}",
            legend { "{member.id}" }
            label { class: "m1-parts-assembly-field", "Component"
                select {
                    value: member.definition_id.as_deref().unwrap_or(""),
                    disabled,
                    onchange: move |event| update_member(value, assets, definition_draft.clone(), definition_change.clone(), definition_id.clone(), MemberPatch::Definition(event.value())),
                    option { value: "", "Visual model only" }
                    for definition in &definitions {
                        option { value: "{definition.id}", "{definition.name}" }
                    }
                }
            }
            if definition.as_ref().is_some_and(|definition| definition.generator.is_some())
                && let Some(parameters) = member.parameters.as_ref() {
                { let checked = parameters.get("hotswap").and_then(serde_json::Value::as_bool).unwrap_or(false);
                  let mut params = parameters.clone();
                  let hotswap_id = member.id.clone(); let hotswap_draft = draft.clone(); let hotswap_change = on_change.clone();
                  rsx! { label { class: "m1-parts-assembly-toggle", "Hotswap socket"
                    input { r#type: "checkbox", checked, disabled, onchange: move |event| {
                        params.insert("hotswap".into(), serde_json::Value::Bool(event.checked()));
                        params.insert("solder".into(), serde_json::Value::Bool(!event.checked()));
                        update_member(value, assets, hotswap_draft.clone(), hotswap_change.clone(), hotswap_id.clone(), MemberPatch::Parameters(Some(params.clone())));
                    } }
                  } }
                }
            }
            label { class: "m1-parts-assembly-field", if member.parameters.as_ref().is_some_and(|parameters| parameters.contains_key("side")) { "Switch side" } else { "Side" }
                select {
                    value: if matches!(&member.side, Side::Back) { "back" } else { "front" },
                    disabled,
                    onchange: move |event| update_member(value, assets, side_draft.clone(), side_change.clone(), side_id.clone(), MemberPatch::Side(event.value() == "back")),
                    option { value: "front", "Front" }
                    option { value: "back", "Back" }
                }
            }
            div { class: "m1-parts-assembly-transform",
                label { "X (mm)" input { r#type: "number", step: "0.1", value: "{member.pose.at.x}", disabled, oninput: move |event| update_member(value, assets, x_draft.clone(), x_change.clone(), x_id.clone(), MemberPatch::X(event.value())) } }
                label { "Y (mm)" input { r#type: "number", step: "0.1", value: "{member.pose.at.y}", disabled, oninput: move |event| update_member(value, assets, y_draft.clone(), y_change.clone(), y_id.clone(), MemberPatch::Y(event.value())) } }
                label { "Angle (°)" input { r#type: "number", step: "1", value: "{member.pose.rotation}", disabled, oninput: move |event| update_member(value, assets, rotation_draft.clone(), rotation_change.clone(), rotation_id.clone(), MemberPatch::Rotation(event.value())) } }
            }
            label { class: "m1-parts-assembly-field", "3D models"
                select {
                    value: if is_custom { "custom" } else { "defaults" },
                    disabled,
                    onchange: move |event| {
                        let custom = event.value() == "custom";
                        let models = if custom && current_models.is_empty() { model_defaults.clone() } else { current_models.clone() };
                        update_member(value, assets, mode_draft.clone(), mode_change.clone(), mode_id.clone(), MemberPatch::ModelMode(custom, models));
                    },
                    option { value: "defaults", "Use component defaults" }
                    option { value: "custom", "Custom model bindings" }
                }
            }
            if is_custom {
                if member.models.is_empty() {
                    p { class: "m1-parts-empty", "No custom models in this member." }
                }
                for (index, model) in member.models.iter().cloned().enumerate() {
                    AssemblyModelFields {
                        key: "{member.id}-{index}",
                        model,
                        index,
                        member_id: member.id.clone(),
                        draft: draft.clone(),
                        value,
                        assets,
                        options: options.clone(),
                        disabled,
                        on_change: on_change.clone(),
                    }
                }
            } else if definition.is_some() {
                p { class: "m1-parts-empty", "Uses {definition_models.len()} component model default(s)." }
                button { r#type: "button", disabled, onclick: move |_| update_member(value, assets, draft.clone(), on_change.clone(), edit_defaults_id.clone(), MemberPatch::ModelMode(true, definition_models.clone())), "Edit model defaults" }
            } else {
                p { class: "m1-parts-empty", "No component model defaults are attached." }
            }
            if is_custom {
                button { r#type: "button", disabled: disabled || options.is_empty(), onclick: move |_| {
                    if let Some(option) = options.first() {
                        update_member(value, assets, draft.clone(), on_change.clone(), add_model_id.clone(), MemberPatch::AddModel(default_model(option.id.clone())));
                    }
                }, "Add model" }
            }
            label { class: "m1-parts-assembly-field", "Import model"
                input { r#type: "file", accept: ".step,.stp,.stl,.wrl", disabled, onchange: import_model }
            }
            if let Some(message) = import_feedback.read().as_ref() {
                p { class: "m1-parts-load-error", role: "alert", "{message}" }
            }
            button { r#type: "button", disabled, onclick: move |_| update_member(value, assets, remove_draft.clone(), remove_change, remove_id.clone(), MemberPatch::RemoveMember), "Remove component" }
        }
    }
}

#[component]
fn AssemblyModelFields(
    model: PartModel,
    index: usize,
    member_id: String,
    draft: AssemblyDraft,
    value: Signal<AssemblyDefinition>,
    assets: Signal<Vec<Asset>>,
    options: Vec<ModelOption>,
    disabled: bool,
    on_change: EventHandler<AssemblyDraft>,
) -> Element {
    let asset_options = options.clone();
    let asset_id = member_id.clone();
    let asset_draft = draft.clone();
    let asset_change = on_change.clone();
    let offset_draft = draft.clone();
    let rotation_draft = draft.clone();
    let scale_draft = draft.clone();
    let offset_change = on_change.clone();
    let rotation_change = on_change.clone();
    let scale_change = on_change.clone();
    let remove_draft = draft.clone();
    let remove_change = on_change.clone();
    let remove_id = member_id.clone();
    rsx! {
        details { class: "m1-parts-assembly-model", open: true,
            summary { "Model {index + 1}" }
            label { class: "m1-parts-assembly-field", "Asset"
                select {
                    value: "{model.asset_id}",
                    disabled,
            onchange: move |event| update_member(value, assets, asset_draft.clone(), asset_change.clone(), asset_id.clone(), MemberPatch::ModelAsset(index, event.value())),
                    for option in &asset_options {
                        option { value: "{option.id}", "{option.name}" }
                    }
                    if !asset_options.iter().any(|option| option.id == model.asset_id) {
                        option { value: "{model.asset_id}", "{model.asset_id}" }
                    }
                }
            }
            ModelVectorFields { model: model.clone(), member_id: member_id.clone(), index, field: ModelVector::Offset, draft: offset_draft, value, assets, disabled, on_change: offset_change }
            ModelVectorFields { model: model.clone(), member_id: member_id.clone(), index, field: ModelVector::Rotation, draft: rotation_draft, value, assets, disabled, on_change: rotation_change }
            ModelVectorFields { model, member_id: member_id.clone(), index, field: ModelVector::Scale, draft: scale_draft, value, assets, disabled, on_change: scale_change }
            button { r#type: "button", disabled, onclick: move |_| update_member(value, assets, remove_draft.clone(), remove_change.clone(), remove_id.clone(), MemberPatch::RemoveModel(index)), "Remove model" }
        }
    }
}

#[component]
fn ModelVectorFields(
    model: PartModel,
    member_id: String,
    index: usize,
    field: ModelVector,
    draft: AssemblyDraft,
    value: Signal<AssemblyDefinition>,
    assets: Signal<Vec<Asset>>,
    disabled: bool,
    on_change: EventHandler<AssemblyDraft>,
) -> Element {
    let vector = model_vector(&model, field);
    let label = match field {
        ModelVector::Offset => "Offset (mm)",
        ModelVector::Rotation => "Rotation (°)",
        ModelVector::Scale => "Scale",
    };
    let axis_x_id = member_id.clone();
    let axis_y_id = member_id.clone();
    let axis_z_id = member_id.clone();
    let x_draft = draft.clone();
    let y_draft = draft.clone();
    let z_draft = draft.clone();
    let x_change = on_change.clone();
    let y_change = on_change.clone();
    let z_change = on_change.clone();
    rsx! {
        div { class: "m1-parts-assembly-model-vector",
            strong { "{label}" }
            div { class: "m1-parts-assembly-transform",
                label { "X" input { r#type: "number", step: "0.1", value: "{vector.x}", disabled, oninput: move |event| update_member(value, assets, x_draft.clone(), x_change.clone(), axis_x_id.clone(), MemberPatch::ModelVector(index, field, ModelAxis::X, event.value())) } }
                label { "Y" input { r#type: "number", step: "0.1", value: "{vector.y}", disabled, oninput: move |event| update_member(value, assets, y_draft.clone(), y_change.clone(), axis_y_id.clone(), MemberPatch::ModelVector(index, field, ModelAxis::Y, event.value())) } }
                label { "Z" input { r#type: "number", step: "0.1", value: "{vector.z}", disabled, oninput: move |event| update_member(value, assets, z_draft.clone(), z_change.clone(), axis_z_id.clone(), MemberPatch::ModelVector(index, field, ModelAxis::Z, event.value())) } }
            }
        }
    }
}

fn publish_draft(
    value: Signal<AssemblyDefinition>,
    assets: Signal<Vec<Asset>>,
    draft: &AssemblyDraft,
    on_change: EventHandler<AssemblyDraft>,
) {
    on_change.call(AssemblyDraft {
        value: value(),
        assets: assets(),
        ..draft.clone()
    });
}

fn update_member(
    mut value: Signal<AssemblyDefinition>,
    assets: Signal<Vec<Asset>>,
    draft: AssemblyDraft,
    on_change: EventHandler<AssemblyDraft>,
    member_id: String,
    patch: MemberPatch,
) {
    let mut updated = false;
    value.with_mut(|assembly| {
        if matches!(&patch, MemberPatch::RemoveMember) {
            let original_len = assembly.members.len();
            assembly.members.retain(|member| member.id != member_id);
            updated = original_len != assembly.members.len();
        } else if let Some(member) = assembly
            .members
            .iter_mut()
            .find(|member| member.id == member_id)
        {
            patch.apply(member);
            updated = true;
        }
    });
    if updated {
        publish_draft(value, assets, &draft, on_change);
    }
}

#[derive(Clone)]
enum MemberPatch {
    Definition(String),
    Side(bool),
    X(String),
    Y(String),
    Rotation(String),
    Parameters(Option<std::collections::BTreeMap<String, serde_json::Value>>),
    ModelMode(bool, Vec<PartModel>),
    AddModel(PartModel),
    RemoveModel(usize),
    ModelAsset(usize, String),
    ModelVector(usize, ModelVector, ModelAxis, String),
    RemoveMember,
}

impl MemberPatch {
    fn apply(self, member: &mut AssemblyMember) {
        match self {
            Self::Definition(id) => {
                member.definition_id = (!id.is_empty()).then_some(id);
                member.parameters = None;
                member.model_mode = Some(boardstudio_core::model::AssemblyModelMode::Defaults);
                member.models.clear();
            }
            Self::Side(back) => {
                member.side = if back { Side::Back } else { Side::Front };
                if let Some(parameters) = member.parameters.as_mut()
                    && parameters.contains_key("side")
                {
                    parameters.insert(
                        "side".into(),
                        serde_json::Value::String(if back { "F" } else { "B" }.into()),
                    );
                }
            }
            Self::X(value) => {
                if let Ok(value) = value.parse::<f64>()
                    && value.is_finite()
                {
                    member.pose.at.x = value;
                }
            }
            Self::Y(value) => {
                if let Ok(value) = value.parse::<f64>()
                    && value.is_finite()
                {
                    member.pose.at.y = value;
                }
            }
            Self::Rotation(value) => {
                if let Ok(value) = value.parse::<f64>()
                    && value.is_finite()
                {
                    member.pose.rotation = value;
                }
            }
            Self::Parameters(parameters) => member.parameters = parameters,
            Self::ModelMode(custom, models) => {
                member.model_mode = Some(if custom {
                    boardstudio_core::model::AssemblyModelMode::Custom
                } else {
                    boardstudio_core::model::AssemblyModelMode::Defaults
                });
                member.models = models;
            }
            Self::AddModel(model) => {
                member.model_mode = Some(boardstudio_core::model::AssemblyModelMode::Custom);
                member.models.push(model);
            }
            Self::RemoveModel(index) => {
                member.model_mode = Some(boardstudio_core::model::AssemblyModelMode::Custom);
                if index < member.models.len() {
                    member.models.remove(index);
                }
            }
            Self::ModelAsset(index, asset_id) => {
                if let Some(model) = member.models.get_mut(index) {
                    model.asset_id = asset_id;
                }
            }
            Self::ModelVector(index, field, axis, raw) => {
                let Ok(value) = raw.parse::<f64>() else {
                    return;
                };
                if !value.is_finite() || (matches!(field, ModelVector::Scale) && value <= 0.0) {
                    return;
                }
                let Some(model) = member.models.get_mut(index) else {
                    return;
                };
                let vector = match field {
                    ModelVector::Offset => &mut model.offset,
                    ModelVector::Rotation => &mut model.rotation,
                    ModelVector::Scale => &mut model.scale,
                };
                match axis {
                    ModelAxis::X => vector.x = value,
                    ModelAxis::Y => vector.y = value,
                    ModelAxis::Z => vector.z = value,
                }
            }
            Self::RemoveMember => { /* handled by update_member's assembly-level branch */ }
        }
    }
}

#[derive(Clone, Copy)]
enum ModelVector {
    Offset,
    Rotation,
    Scale,
}
#[derive(Clone, Copy)]
enum ModelAxis {
    X,
    Y,
    Z,
}

fn model_vector(model: &PartModel, field: ModelVector) -> Vec3 {
    match field {
        ModelVector::Offset => model.offset,
        ModelVector::Rotation => model.rotation,
        ModelVector::Scale => model.scale,
    }
}

#[derive(Clone)]
struct ModelOption {
    id: String,
    name: String,
}

fn model_options(document_assets: &[Asset], draft_assets: &[Asset]) -> Vec<ModelOption> {
    let is_model = |name: &str| {
        [".step", ".stp", ".stl", ".wrl"]
            .iter()
            .any(|extension| name.to_ascii_lowercase().ends_with(extension))
    };
    let mut options = document_assets
        .iter()
        .chain(draft_assets)
        .filter(|asset| is_model(&asset.name))
        .map(|asset| ModelOption {
            id: asset.id.clone(),
            name: asset.name.clone(),
        })
        .collect::<Vec<_>>();
    for (id, _) in crate::bundled_models::preview_model_paths() {
        if options.iter().any(|option| option.id == id) {
            continue;
        }
        if let Some(model) = crate::bundled_models::bundled_model(id) {
            options.push(ModelOption {
                id: id.to_owned(),
                name: model.filename.to_owned(),
            });
        }
    }
    options
}

fn default_model(asset_id: String) -> PartModel {
    PartModel {
        asset_id,
        offset: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        rotation: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        scale: Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    }
}

fn assembly_preview_recipe(
    assembly: &AssemblyDefinition,
    definitions: &[PartDefinition],
    assets: &[Asset],
) -> Vec<crate::parts_preview::PartsPreviewRecipeMember> {
    assembly
        .members
        .iter()
        .enumerate()
        .map(|(index, member)| {
            let mut definition = member
                .definition_id
                .as_deref()
                .and_then(|id| definitions.iter().find(|definition| definition.id == id))
                .cloned()
                .unwrap_or_else(|| model_only_definition(member));
            if let Some(generator) = definition.generator.as_mut()
                && let Some(parameters) = member.parameters.as_ref()
            {
                generator.parameters.extend(parameters.clone());
            }
            if matches!(
                &member.model_mode,
                Some(boardstudio_core::model::AssemblyModelMode::Custom)
            ) || (member.model_mode.is_none() && !member.models.is_empty())
            {
                definition.models = Some(member.models.clone());
            }
            crate::parts_preview::PartsPreviewRecipeMember {
                id: member.id.clone(),
                definition,
                assets: assets.to_vec(),
                at: member.pose.at,
                rotation: member.pose.rotation,
                side: member.side.clone(),
                generator_parameters: member.parameters.clone().unwrap_or_default(),
            }
        })
        .collect()
}

fn model_only_definition(member: &AssemblyMember) -> PartDefinition {
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: format!("assembly-model-only:{}", member.id),
        name: format!("{} model", member.id),
        kind: PartKind::Custom,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        courtyard: Vec::new(),
        pads: Vec::new(),
        models: Some(member.models.clone()),
        generator: None,
        mechanical_profile: None,
    }
}

fn assembly_owner_current(
    runtime: &crate::runtime::Runtime,
    draft: &AssemblyDraft,
    workspace: &'static str,
) -> bool {
    if workspace != "Parts" || runtime.scope() != draft.scope {
        return false;
    }
    let model = runtime.model();
    model.accepted.as_ref().is_some_and(|accepted| {
        accepted.document.id == draft.document_id && accepted.session_epoch == draft.session_epoch
    })
}

fn unique_asset_id(root: &str, document_assets: &[Asset], draft_assets: &[Asset]) -> String {
    let used = |id: &str| {
        document_assets
            .iter()
            .chain(draft_assets)
            .any(|asset| asset.id == id)
    };
    if !used(root) {
        return root.to_owned();
    }
    (2..=1024)
        .map(|suffix| format!("{root}-{suffix}"))
        .find(|id| !used(id))
        .unwrap_or_else(|| format!("{root}-new"))
}

fn saved_document(
    accepted: &ProjectDoc,
    draft: &AssemblyDraft,
    definitions: &[PartDefinition],
) -> Result<ProjectDoc, String> {
    if draft.value.name.trim().is_empty() || draft.value.members.is_empty() {
        return Err("Name the assembly and add at least one member".into());
    }
    let member_ids = draft
        .value
        .members
        .iter()
        .map(|member| member.id.as_str())
        .collect::<BTreeSet<_>>();
    if member_ids.len() != draft.value.members.len()
        || member_ids.iter().any(|id| id.trim().is_empty())
    {
        return Err("Assembly members must have unique, non-empty identities.".into());
    }
    if draft.value.members.iter().any(|member| {
        !member.pose.at.x.is_finite()
            || !member.pose.at.y.is_finite()
            || !member.pose.rotation.is_finite()
    }) {
        return Err("Assembly member positions and angles must be finite.".into());
    }
    let model_asset_exists = |asset_id: &str| {
        accepted
            .assets
            .iter()
            .chain(&draft.assets)
            .any(|asset| asset.id == asset_id)
            || crate::bundled_models::bundled_model(asset_id).is_some()
    };
    for member in &draft.value.members {
        for model in &member.models {
            let finite = [
                model.offset.x,
                model.offset.y,
                model.offset.z,
                model.rotation.x,
                model.rotation.y,
                model.rotation.z,
                model.scale.x,
                model.scale.y,
                model.scale.z,
            ]
            .into_iter()
            .all(f64::is_finite);
            if !finite
                || [model.scale.x, model.scale.y, model.scale.z]
                    .into_iter()
                    .any(|scale| scale <= 0.0)
            {
                return Err("Model positions and rotations must be finite, and model scales must be positive.".into());
            }
            if !model_asset_exists(&model.asset_id) {
                return Err(format!(
                    "The model asset '{}' is not available in this project.",
                    model.asset_id
                ));
            }
        }
    }
    let current = accepted
        .assemblies
        .iter()
        .find(|assembly| assembly.id == draft.value.id);
    match (&draft.base, current) {
        (Some(base), Some(current)) if current == base => {}
        (None, None) => {}
        (Some(_), _) => {
            return Err(
                "This saved assembly changed while the editor was open. Reopen it before saving."
                    .into(),
            );
        }
        (None, Some(_)) => {
            return Err(
                "An assembly with this identity already exists. Close the editor and try again."
                    .into(),
            );
        }
    }

    let mut document = accepted.clone();
    for asset in &draft.assets {
        if let Some(existing) = document
            .assets
            .iter()
            .find(|existing| existing.id == asset.id)
        {
            if existing != asset {
                return Err("A different project asset already uses this model identity.".into());
            }
        } else {
            document.assets.push(asset.clone());
        }
    }
    document
        .assemblies
        .retain(|assembly| assembly.id != draft.value.id);
    let assembly = draft.value.clone();
    for member in &assembly.members {
        let Some(definition_id) = member.definition_id.as_deref() else {
            continue;
        };
        if document
            .definitions
            .iter()
            .any(|definition| definition.id == definition_id)
        {
            continue;
        }
        if let Some(definition) = definitions
            .iter()
            .find(|definition| definition.id == definition_id)
        {
            document.definitions.push(definition.clone());
        }
    }
    document.assemblies.push(assembly);
    Ok(document)
}

fn next_id(document: &ProjectDoc) -> Result<String, String> {
    let base = browser_uuid().unwrap_or_else(|| format!("{:x}", Date::now().max(0.0) as u64));
    let used = |candidate: &str| {
        document
            .assemblies
            .iter()
            .any(|assembly| assembly.id == candidate)
    };
    let root = format!("assembly-{base}");
    if !used(&root) {
        return Ok(root);
    }
    (2..=1024)
        .map(|suffix| format!("{root}-{suffix}"))
        .find(|candidate| !used(candidate))
        .ok_or_else(|| "Could not allocate a unique assembly identity.".into())
}

fn browser_uuid() -> Option<String> {
    let crypto = Reflect::get(&js_sys::global(), &JsValue::from_str("crypto")).ok()?;
    let random_uuid = Reflect::get(&crypto, &JsValue::from_str("randomUUID"))
        .ok()?
        .dyn_into::<Function>()
        .ok()?;
    random_uuid.call0(&crypto).ok()?.as_string()
}

fn member_id(assembly: &AssemblyDefinition) -> Option<String> {
    let current = Date::now().max(0.0) as u64;
    let root = browser_uuid().unwrap_or_else(|| format!("{current:x}"));
    let used = |candidate: &str| assembly.members.iter().any(|member| member.id == candidate);
    let candidate = format!("component-{}", &root[..root.len().min(8)]);
    if !used(&candidate) {
        return Some(candidate);
    }
    (2..=1024)
        .map(|suffix| format!("{candidate}-{suffix}"))
        .find(|id| !used(id))
}
