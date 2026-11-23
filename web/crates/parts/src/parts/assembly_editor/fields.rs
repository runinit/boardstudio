use super::super::model_transform::ModelVec3Fields;
use super::*;
use boardstudio_web_ui_shared::inspector_fields::{
    FilePickerButton, InspectorSection, TOUCH_CLS, ToggleField,
};

#[component]
pub(super) fn AssemblyDraftFields(
    snapshot: AcceptedSnapshot,
    draft: AssemblyDraft,
    definitions: Vec<PartDefinition>,
    place_pending: bool,
    matrix_pending: bool,
    selected_context: Signal<Option<super::super::super::objects::ScopedTreeContext>>,
    on_place: EventHandler<super::super::super::objects::MatrixPlacementSource>,
    on_place_board: EventHandler<(AssemblyDraft, f64, f64)>,
    on_apply: EventHandler<AssemblyDefinition>,
    on_change: EventHandler<AssemblyDraft>,
    on_save: EventHandler<MouseEvent>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    let value = use_signal(|| draft.value.clone());
    let assets = use_signal(|| draft.assets.clone());
    let mut origin_x = use_signal(|| 0.0_f64);
    let mut origin_y = use_signal(|| 0.0_f64);
    let import_pending = use_signal(|| false);
    let render_value = value();
    let apply_value = render_value.clone();
    let placement_value = render_value.clone();
    let member_definitions = definitions
        .iter()
        .filter(|definition| {
            !matches!(
                &definition.kind,
                boardstudio_model::model::PartKind::Utility
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let recipe = assembly_preview_recipe(&render_value, &member_definitions, &assets());
    let preview_definition = recipe
        .first()
        .map(|member| Rc::new(member.definition.clone()));
    let preview_identity = serde_json::to_string(&render_value).unwrap_or_default();
    let controls_disabled = import_pending();
    let unsaved_assets = assets().iter().any(|asset| {
        !snapshot
            .document
            .assets
            .iter()
            .any(|accepted| accepted == asset)
    });
    let matrix_selected = selected_context().as_ref().is_some_and(|selected| {
        draft.scope.as_ref() == Some(&selected.scope)
            && matches!(
                &selected.context,
                super::super::super::objects::TreeContext::Matrix { .. }
            )
    });
    let placement_definitions = snapshot
        .document
        .definitions
        .iter()
        .chain(member_definitions.iter())
        .cloned()
        .collect::<Vec<_>>();
    rsx! {
        section { class: "grid gap-3", "aria-label": "Assembly editor",
            h3 { "Assembly editor" }
            { let name_draft = draft.clone();
              rsx! { TextCommitField { label: "Name", value: "{render_value.name}", disabled_reason: controls_disabled.then(|| "Assembly is read-only".to_owned()), on_commit: move |name| { let mut value = value; value.with_mut(|value| value.name = name); publish_draft(value, assets, &name_draft, on_change); } } }
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
                    on_change,
                }
            }
            { let add_draft = draft.clone();
              let mut add_value = value; let add_assets = assets;
              rsx! { Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled: controls_disabled, onclick: move |_| {
                  if let Some(id) = member_id(&add_value()) {
                      add_value.with_mut(|value| value.members.push(AssemblyMember {
                          parameters: None,
                          model_mode: Some(boardstudio_model::model::AssemblyModelMode::Defaults),
                          id,
                          definition_id: None,
                          pose: Pose2 { at: Vec2 { x: 0.0, y: 0.0 }, rotation: 0.0 },
                          side: Side::Front,
                          models: Vec::new(),
                      }));
                      publish_draft(add_value, add_assets, &add_draft, on_change);
                  }
              }, "Add component" } }
            }
            div { class: "flex flex-wrap gap-2",
                Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled: controls_disabled, onclick: on_save, "Save assembly" }
                Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled: controls_disabled, onclick: on_close, "Close editor" }
            }
            div { class: "flex flex-wrap gap-2",
                Button {
                    variant: ButtonVariant::Secondary,
                    class: TOUCH_CLS,
                    disabled: controls_disabled || unsaved_assets,
                    onclick: move |_| on_place.call(super::super::super::objects::MatrixPlacementSource::Assembly {
                        assembly: placement_value.clone(),
                        definitions: placement_definitions.clone(),
                    }),
                    "Place assembly in Layout"
                }
                if matrix_selected {
                    Button {
                        variant: ButtonVariant::Secondary,
                        class: TOUCH_CLS,
                        disabled: controls_disabled || matrix_pending || unsaved_assets,
                        onclick: move |_| on_apply.call(apply_value.clone()),
                        "Apply to selected matrix"
                    }
                }
            }
            fieldset { class: "grid gap-2",
                legend { "Place assembly" }
                div { class: "grid grid-cols-[repeat(auto-fit,minmax(min(100%,190px),1fr))] gap-2",
                    NumericField {
                        label: "X (mm)",
                        step: Some(0.1),
                        value: origin_x(),
                        disabled_reason: controls_disabled.then(|| "Assembly is read-only".to_owned()),
                        on_commit: move |value: f64| origin_x.set(value),
                    }
                    NumericField {
                        label: "Y (mm)",
                        step: Some(0.1),
                        value: origin_y(),
                        disabled_reason: controls_disabled.then(|| "Assembly is read-only".to_owned()),
                        on_commit: move |value: f64| origin_y.set(value),
                    }
                }
                { let place_render = render_value.clone();
                  let place_draft = draft.clone();
                  rsx! { Button {
                    variant: ButtonVariant::Secondary,
                    class: TOUCH_CLS,
                    disabled: controls_disabled || place_pending || draft.scope.is_none() || render_value.members.is_empty(),
                    onclick: move |_| on_place_board.call((
                        AssemblyDraft {
                            value: place_render.clone(),
                            assets: assets(),
                            ..place_draft.clone()
                        },
                        origin_x(),
                        origin_y(),
                    )),
                    "Place on selected board"
                } } }
            }
            if unsaved_assets {
                div { role: "status", Callout { variant: CalloutVariant::Info, "Save imported model assets before applying this assembly to a matrix." } }
            }
            if !recipe.is_empty() {
                div { class: "grid gap-2",
                    super::super::PartsPreviewPanel {
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
                div { role: "status", Callout { variant: CalloutVariant::Info, "Add a component or model to preview this assembly." } }
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
    let workspace = use_context::<super::super::super::WorkspaceState>().0;
    let import_feedback = use_signal(|| None::<String>);
    let definition = member
        .definition_id
        .as_deref()
        .and_then(|id| definitions.iter().find(|definition| definition.id == id))
        .cloned();
    let definition_options = definitions
        .iter()
        .map(|definition| ModelOption {
            id: definition.id.clone(),
            name: definition.name.clone(),
        })
        .collect::<Vec<_>>();
    let options = model_options(&snapshot.document.assets, &assets());
    let is_custom = matches!(
        &member.model_mode,
        Some(boardstudio_model::model::AssemblyModelMode::Custom)
    ) || (member.model_mode.is_none() && !member.models.is_empty());
    let definition_models = definition
        .as_ref()
        .and_then(|definition| definition.models.clone())
        .unwrap_or_default();
    let model_defaults = definition_models.clone();
    let current_models = member.models.clone();
    let edit = use_callback({
        let draft = draft.clone();
        let id = member.id.clone();
        move |patch| update_member(value, assets, draft.clone(), on_change, id.clone(), patch)
    });
    let mut import_model = {
        let runtime = runtime.clone();
        let draft = draft.clone();
        let mut import_pending = import_pending;
        let mut import_feedback = import_feedback;
        move |file: web_sys::File| {
            if disabled || import_pending() {
                return;
            }
            import_pending.set(true);
            import_feedback.set(None);
            let runtime = runtime.clone();
            let draft = draft.clone();
            let mut assets = assets;
            let mut import_pending = import_pending;
            let mut import_feedback = import_feedback;
            spawn(async move {
                let result = async {
                    let imported =
                        crate::presentation::model_asset_import::read_model_file(file).await?;
                    if !assembly_owner_current(&runtime, &draft, workspace()) {
                        return Err(
                            "The project or Parts scope changed while reading the model."
                                .to_owned(),
                        );
                    }
                    imported.store(&runtime.store).await?;
                    if !assembly_owner_current(&runtime, &draft, workspace()) {
                        return Err(
                            "The project or Parts scope changed while storing the model."
                                .to_owned(),
                        );
                    }
                    let current = runtime
                        .model()
                        .accepted
                        .ok_or("The accepted project is unavailable.")?;
                    Ok((imported, current))
                }
                .await;
                import_pending.set(false);
                let (imported, current) = match result {
                    Ok(value) => value,
                    Err(message) => {
                        import_feedback.set(Some(message));
                        return;
                    }
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
                edit.call(MemberPatch::AddModel(new_model(asset.id)));
            });
        }
    };

    let edit_defaults = {
        let runtime = runtime.clone();
        let draft = draft.clone();
        let definition = definition.clone();
        let member = member.clone();
        let mut import_pending = import_pending;
        let mut import_feedback = import_feedback;
        move |_| {
            let Some(mut definition) = definition.clone() else {
                return;
            };
            if disabled || import_pending() {
                return;
            }
            import_pending.set(true);
            import_feedback.set(None);
            let runtime = runtime.clone();
            let draft = draft.clone();
            let member = member.clone();
            let mut import_pending = import_pending;
            let mut import_feedback = import_feedback;
            spawn(async move {
                if !assembly_owner_current(&runtime, &draft, workspace()) {
                    import_feedback.set(Some("The project or Parts scope changed. Reopen the assembly before editing model defaults.".into()));
                    import_pending.set(false);
                    return;
                }
                let mut generator_parameters = member.parameters.clone().unwrap_or_default();
                if let Some(generator) = definition.generator.as_mut() {
                    generator.parameters.extend(generator_parameters.clone());
                }
                let part = Part {
                    keycap: None,
                    outline: None,
                    id: member.id.clone(),
                    definition_id: definition.id.clone(),
                    reference: member.id.clone(),
                    pose: Pose2 {
                        at: Vec2::default(),
                        rotation: 0.0,
                    },
                    side: member.side.clone(),
                    anchor: None,
                    locked: None,
                    properties: None,
                    generator_parameters: Some(std::mem::take(&mut generator_parameters)),
                };
                let models = match crate::bundled_models::model_bindings(&definition, &part).await {
                    Ok(models) => models,
                    Err(error) => {
                        import_feedback.set(Some(error));
                        import_pending.set(false);
                        return;
                    }
                };
                if !assembly_owner_current(&runtime, &draft, workspace()) {
                    import_pending.set(false);
                    return;
                }
                edit.call(MemberPatch::ModelMode(true, models));
                import_feedback.set(None);
                import_pending.set(false);
            });
        }
    };

    rsx! {
        fieldset { class: "grid gap-2", key: "{member.id}",
            legend { "{member.id}" }
            SelectField::<String> {
                label: "Component",
                value: (member.definition_id.as_deref().unwrap_or("")).to_string(),
                options: component_option_rows(&definition_options, member.definition_id.as_deref()),
                disabled,
                on_commit: move |selected: String| edit.call(MemberPatch::Definition(selected)),
            }
            if definition.as_ref().is_some_and(|definition| definition.generator.is_some())
                && let Some(parameters) = member.parameters.as_ref() {
                { let checked = parameters.get("hotswap").and_then(serde_json::Value::as_bool).unwrap_or(false);
                  let mut params = parameters.clone();
                  rsx! { ToggleField {
                    label: "Hotswap socket", checked,
                    disabled_reason: disabled.then(|| "Assembly is read-only".to_owned()),
                    on_commit: move |enabled| {
                        params.insert("hotswap".into(), serde_json::Value::Bool(enabled));
                        params.insert("solder".into(), serde_json::Value::Bool(!enabled));
                        edit.call(MemberPatch::Parameters(Some(params.clone())));
                    },
                  } }
                }
            }
            SelectField::<String> {
                label: if member.parameters.as_ref().is_some_and(|parameters| parameters.contains_key("side")) { "Switch side".to_owned() } else { "Side".to_owned() },
                value: if matches!(&member.side, Side::Back) { "back".to_owned() } else { "front".to_owned() },
                options: vec![("front".to_owned(), "Front".to_owned()), ("back".to_owned(), "Back".to_owned())],
                disabled,
                on_commit: move |selected: String| edit.call(MemberPatch::Side(selected == "back")),
            }
            div { class: "grid grid-cols-3 gap-2",
                NumericField { label: "X (mm)", step: Some(0.1), value: member.pose.at.x, disabled_reason: disabled.then(|| "Assembly is read-only".to_owned()), on_commit: move |number: f64| edit.call(MemberPatch::X(number)) }
                NumericField { label: "Y (mm)", step: Some(0.1), value: member.pose.at.y, disabled_reason: disabled.then(|| "Assembly is read-only".to_owned()), on_commit: move |number: f64| edit.call(MemberPatch::Y(number)) }
                NumericField { label: "Angle (°)", step: Some(1.0), value: member.pose.rotation, disabled_reason: disabled.then(|| "Assembly is read-only".to_owned()), on_commit: move |number: f64| edit.call(MemberPatch::Rotation(number)) }
            }
            SelectField::<String> {
                label: "3D models",
                value: (if is_custom { "custom" } else { "defaults" }).to_string(),
                options: vec![("defaults".to_owned(), "Use component defaults".to_owned()), ("custom".to_owned(), "Custom model bindings".to_owned())],
                disabled,
                on_commit: move |selected: String| {
                    let custom = selected == "custom";
                    let models = if custom && current_models.is_empty() { model_defaults.clone() } else { current_models.clone() };
                    edit.call(MemberPatch::ModelMode(custom, models));
                },
            }
            if is_custom {
                if member.models.is_empty() {
                    p { "No custom models in this member." }
                }
                for (index, model) in member.models.iter().cloned().enumerate() {
                    AssemblyModelFields {
                        key: "{member.id}-{index}",
                        model,
                        index,
                        options: options.clone(),
                        disabled,
                        edit,
                    }
                }
            } else if definition.is_some() {
                p { "Uses {definition_models.len()} component model default(s)." }
                Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled, onclick: edit_defaults, "Edit model defaults" }
            } else {
                p { "No component model defaults are attached." }
            }
            if is_custom {
                Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled: disabled || options.is_empty(), onclick: move |_| {
                    if let Some(option) = options.first() {
                        edit.call(MemberPatch::AddModel(new_model(option.id.clone())));
                    }
                }, "Add model" }
            }
            FilePickerButton {
                label: "Import model",
                accept: ".step,.stp,.stl,.wrl",
                disabled,
                on_files: move |files: Vec<web_sys::File>| if let Some(file) = files.into_iter().next() { import_model(file); },
            }
            if let Some(message) = import_feedback.read().as_ref() {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            }
            Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled, onclick: move |_| edit.call(MemberPatch::RemoveMember), "Remove component" }
        }
    }
}

#[component]
fn AssemblyModelFields(
    model: PartModel,
    index: usize,
    options: Vec<ModelOption>,
    disabled: bool,
    edit: EventHandler<MemberPatch>,
) -> Element {
    rsx! {
        InspectorSection { title: "Model {index + 1}",
            SelectField::<String> {
                label: "Asset", value: "{model.asset_id}",
                options: model_asset_option_rows(&options, &model.asset_id),
                disabled,
                on_commit: move |selected| edit.call(MemberPatch::ModelAsset(index, selected)),
            }
            for (label, field) in [("Offset (mm)", ModelVector::Offset), ("Rotation (°)", ModelVector::Rotation), ("Scale", ModelVector::Scale)] {
                fieldset {
                    legend { "{label}" }
                    ModelVec3Fields { value: model_vector(&model, field),
                        labels: ["X".into(), "Y".into(), "Z".into()],
                        disabled_reason: disabled.then(|| "Assembly is read-only".to_owned()),
                        on_commit: move |(axis, number)| edit.call(MemberPatch::ModelVector(index, field, axis, number)),
                    }
                }
            }
            Button { disabled, onclick: move |_| edit.call(MemberPatch::RemoveModel(index)), "Remove model" }
        }
    }
}
