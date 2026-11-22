use super::*;
use boardstudio_web_ui_shared::inspector_fields::{CheckboxField, InspectorSection};

#[component]
pub(super) fn AssemblyDraftFields(
    snapshot: AcceptedSnapshot,
    draft: AssemblyDraft,
    definitions: Vec<PartDefinition>,
    place_pending: bool,
    matrix_pending: bool,
    selected_context: Signal<Option<super::super::super::objects::ScopedTreeContext>>,
    on_place: EventHandler<super::super::super::objects::MatrixPlacementSource>,
    on_place_board: EventHandler<(AssemblyDraft, String, String)>,
    on_apply: EventHandler<AssemblyDefinition>,
    on_change: EventHandler<AssemblyDraft>,
    on_save: EventHandler<MouseEvent>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    let value = use_signal(|| draft.value.clone());
    let assets = use_signal(|| draft.assets.clone());
    let mut origin_x = use_signal(|| "0".to_owned());
    let mut origin_y = use_signal(|| "0".to_owned());
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
              rsx! { EditorTextField { label: "Name", value: "{render_value.name}", disabled: controls_disabled, oninput: move |event: FormEvent| { let mut value = value; value.with_mut(|value| value.name = event.value()); publish_draft(value, assets, &name_draft, on_change); } } }
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
              rsx! { Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: controls_disabled, onclick: move |_| {
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
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: controls_disabled, onclick: on_save, "Save assembly" }
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: controls_disabled, onclick: on_close, "Close editor" }
            }
            div { class: "flex flex-wrap gap-2",
                Button {
                    variant: ButtonVariant::Secondary,
                    class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
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
                        class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: controls_disabled || matrix_pending || unsaved_assets,
                        onclick: move |_| on_apply.call(apply_value.clone()),
                        "Apply to selected matrix"
                    }
                }
            }
            fieldset { class: "grid gap-2",
                legend { "Place assembly" }
                div { class: "grid grid-cols-[repeat(auto-fit,minmax(min(100%,190px),1fr))] gap-2",
                    EditorNumberField {
                        label: "X (mm)",
                        step: "0.1",
                        value: "{origin_x}",
                        disabled: controls_disabled,
                        oninput: move |event: FormEvent| origin_x.set(event.value()),
                    }
                    EditorNumberField {
                        label: "Y (mm)",
                        step: "0.1",
                        value: "{origin_y}",
                        disabled: controls_disabled,
                        oninput: move |event: FormEvent| origin_y.set(event.value()),
                    }
                }
                { let place_render = render_value.clone();
                  let place_draft = draft.clone();
                  rsx! { Button {
                    variant: ButtonVariant::Secondary,
                    class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
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
    let mut import_input = use_signal(|| None::<HtmlInputElement>);
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
    let component_id = member.id.clone();
    let definition_id = member.id.clone();
    let side_id = member.id.clone();
    let x_id = member.id.clone();
    let y_id = member.id.clone();
    let rotation_id = member.id.clone();
    let mode_id = member.id.clone();
    let remove_id = member.id.clone();
    let add_model_id = member.id.clone();
    let definition_draft = draft.clone();
    let side_draft = draft.clone();
    let x_draft = draft.clone();
    let y_draft = draft.clone();
    let rotation_draft = draft.clone();
    let mode_draft = draft.clone();
    let remove_draft = draft.clone();
    let mut component_value = value;
    let add_model_draft = draft.clone();

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
        let assets = assets;
        let mut import_pending = import_pending;
        let mut import_feedback = import_feedback;
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
            let mut assets = assets;
            let mut import_pending = import_pending;
            let mut import_feedback = import_feedback;
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
                            Some(boardstudio_model::model::AssemblyModelMode::Custom);
                        member.models.push(default_model(asset.id.clone()));
                    }
                });
                publish_draft(component_value, assets, &draft, on_change);
                import_feedback.set(None);
                import_pending.set(false);
            });
        }
    };

    let edit_defaults = {
        let runtime = runtime.clone();
        let mounted = mounted.clone();
        let draft = draft.clone();
        let definition = definition.clone();
        let member = member.clone();
        let value = value;
        let assets = assets;
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
            let mounted = mounted.clone();
            let draft = draft.clone();
            let member = member.clone();
            let mut value = value;
            let assets = assets;
            let mut import_pending = import_pending;
            let mut import_feedback = import_feedback;
            spawn_local(async move {
                if !assembly_owner_current(&runtime, &draft, workspace()) {
                    if mounted.get() {
                        import_feedback.set(Some("The project or Parts scope changed. Reopen the assembly before editing model defaults.".into()));
                    }
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
                        if mounted.get() {
                            import_feedback.set(Some(error));
                        }
                        import_pending.set(false);
                        return;
                    }
                };
                if !mounted.get() || !assembly_owner_current(&runtime, &draft, workspace()) {
                    import_pending.set(false);
                    return;
                }
                value.with_mut(|assembly| {
                    if let Some(member) = assembly
                        .members
                        .iter_mut()
                        .find(|candidate| candidate.id == member.id)
                    {
                        member.model_mode =
                            Some(boardstudio_model::model::AssemblyModelMode::Custom);
                        member.models = models;
                    }
                });
                publish_draft(value, assets, &draft, on_change);
                import_feedback.set(None);
                import_pending.set(false);
            });
        }
    };

    rsx! {
        fieldset { class: "grid gap-2", key: "{component_id}",
            legend { "{member.id}" }
            EditorChoiceField {
                label: "Component",
                value: (member.definition_id.as_deref().unwrap_or("")).to_string(),
                options: component_option_rows(&definition_options, member.definition_id.as_deref()),
                disabled,
                on_select: move |selected: String| update_member(value, assets, definition_draft.clone(), on_change, definition_id.clone(), MemberPatch::Definition(selected)),
            }
            if definition.as_ref().is_some_and(|definition| definition.generator.is_some())
                && let Some(parameters) = member.parameters.as_ref() {
                { let checked = parameters.get("hotswap").and_then(serde_json::Value::as_bool).unwrap_or(false);
                  let mut params = parameters.clone();
                  let hotswap_id = member.id.clone(); let hotswap_draft = draft.clone();
                  rsx! { CheckboxField {
                    id: format!("assembly-hotswap-{}", dioxus::core::current_scope_id().0),
                    label: "Hotswap socket",
                    aria_label: Some("Hotswap socket".to_owned()),
                    disabled,
                    checked: if checked { CheckboxState::Checked } else { CheckboxState::Unchecked },
                    on_checked_change: move |state: CheckboxState| {
                        let enabled = bool::from(state);
                        params.insert("hotswap".into(), serde_json::Value::Bool(enabled));
                        params.insert("solder".into(), serde_json::Value::Bool(!enabled));
                        update_member(value, assets, hotswap_draft.clone(), on_change, hotswap_id.clone(), MemberPatch::Parameters(Some(params.clone())));
                    },
                  } }
                }
            }
            EditorSelect {
                label: if member.parameters.as_ref().is_some_and(|parameters| parameters.contains_key("side")) { "Switch side".to_owned() } else { "Side".to_owned() },
                value: if matches!(&member.side, Side::Back) { "back".to_owned() } else { "front".to_owned() },
                options: vec![("front".to_owned(), "Front".to_owned()), ("back".to_owned(), "Back".to_owned())],
                disabled,
                on_select: move |selected: String| update_member(value, assets, side_draft.clone(), on_change, side_id.clone(), MemberPatch::Side(selected == "back")),
            }
            div { class: "grid grid-cols-3 gap-2",
                EditorNumberField { label: "X (mm)", step: "0.1", value: "{member.pose.at.x}", disabled, oninput: move |event: FormEvent| update_member(value, assets, x_draft.clone(), on_change, x_id.clone(), MemberPatch::X(event.value())) }
                EditorNumberField { label: "Y (mm)", step: "0.1", value: "{member.pose.at.y}", disabled, oninput: move |event: FormEvent| update_member(value, assets, y_draft.clone(), on_change, y_id.clone(), MemberPatch::Y(event.value())) }
                EditorNumberField { label: "Angle (°)", step: "1", value: "{member.pose.rotation}", disabled, oninput: move |event: FormEvent| update_member(value, assets, rotation_draft.clone(), on_change, rotation_id.clone(), MemberPatch::Rotation(event.value())) }
            }
            EditorSelect {
                label: "3D models",
                value: (if is_custom { "custom" } else { "defaults" }).to_string(),
                options: vec![("defaults".to_owned(), "Use component defaults".to_owned()), ("custom".to_owned(), "Custom model bindings".to_owned())],
                disabled,
                on_select: move |selected: String| {
                    let custom = selected == "custom";
                    let models = if custom && current_models.is_empty() { model_defaults.clone() } else { current_models.clone() };
                    update_member(value, assets, mode_draft.clone(), on_change, mode_id.clone(), MemberPatch::ModelMode(custom, models));
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
                        member_id: member.id.clone(),
                        draft: draft.clone(),
                        value,
                        assets,
                        options: options.clone(),
                        disabled,
                        on_change,
                    }
                }
            } else if definition.is_some() {
                p { "Uses {definition_models.len()} component model default(s)." }
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled, onclick: edit_defaults, "Edit model defaults" }
            } else {
                p { "No component model defaults are attached." }
            }
            if is_custom {
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: disabled || options.is_empty(), onclick: move |_| {
                    if let Some(option) = options.first() {
                        update_member(value, assets, add_model_draft.clone(), on_change, add_model_id.clone(), MemberPatch::AddModel(default_model(option.id.clone())));
                    }
                }, "Add model" }
            }
            Button {
                variant: ButtonVariant::Secondary,
                class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                disabled,
                onclick: move |_| {
                    if let Some(input) = import_input() {
                        input.click();
                    }
                },
                "Import model"
            }
            input {
                r#type: "file",
                hidden: true,
                accept: ".step,.stp,.stl,.wrl",
                aria_label: "Import model",
                disabled,
                onmounted: move |event: MountedEvent| {
                    import_input.set(event.data().downcast::<web_sys::Element>().and_then(|e| e.dyn_ref::<HtmlInputElement>()).cloned());
                },
                onchange: import_model,
            }
            if let Some(message) = import_feedback.read().as_ref() {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            }
            Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled, onclick: move |_| update_member(value, assets, remove_draft.clone(), on_change, remove_id.clone(), MemberPatch::RemoveMember), "Remove component" }
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
    let offset_draft = draft.clone();
    let rotation_draft = draft.clone();
    let scale_draft = draft.clone();
    let remove_draft = draft.clone();
    let remove_id = member_id.clone();
    let mut model_open = use_signal(|| true);
    rsx! {
        InspectorSection {
            title: "Model {index + 1}",
            open: model_open(),
            on_open_change: move |next: bool| model_open.set(next),
            EditorChoiceField {
                label: "Asset",
                value: "{model.asset_id}",
                options: model_asset_option_rows(&asset_options, &model.asset_id),
                disabled,
                on_select: move |selected: String| update_member(value, assets, asset_draft.clone(), on_change, asset_id.clone(), MemberPatch::ModelAsset(index, selected)),
            }
            ModelVectorFields { model: model.clone(), member_id: member_id.clone(), index, field: ModelVector::Offset, draft: offset_draft, value, assets, disabled, on_change }
            ModelVectorFields { model: model.clone(), member_id: member_id.clone(), index, field: ModelVector::Rotation, draft: rotation_draft, value, assets, disabled, on_change }
            ModelVectorFields { model: model.clone(), member_id: member_id.clone(), index, field: ModelVector::Scale, draft: scale_draft, value, assets, disabled, on_change }
            Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled, onclick: move |_| update_member(value, assets, remove_draft.clone(), on_change, remove_id.clone(), MemberPatch::RemoveModel(index)), "Remove model" }
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
    let axes = [
        ("X", ModelAxis::X, vector.x),
        ("Y", ModelAxis::Y, vector.y),
        ("Z", ModelAxis::Z, vector.z),
    ];
    rsx! {
        div { class: "grid gap-1",
            strong { "{label}" }
            ModelAxisGrid {
                for (axis_label, axis, axis_value) in axes {
                    {
                        let member_id = member_id.clone();
                        let draft = draft.clone();
                        rsx! {
                            ModelAxisField {
                                axis: axis_label,
                                value: axis_value.to_string(),
                                disabled,
                                oninput: move |event: FormEvent| update_member(value, assets, draft.clone(), on_change, member_id.clone(), MemberPatch::ModelVector(index, field, axis, event.value()))
                            }
                        }
                    }
                }
            }
        }
    }
}
