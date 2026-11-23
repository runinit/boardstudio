//! Parts-owned saved assembly list and the first authoring slice of F4.6.
use super::catalogue::CatalogEntry;
use super::model_transform::{
    ModelAxis, ModelVector, model_vector, new_model, set_model_axis, valid_transform_value,
};
use crate::parts_custom_definition::{replace_document_resolver, replacement_commit};
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope,
};
use boardstudio_model::model::{
    AssemblyDefinition, AssemblyMember, AssemblyModelMode, Asset, EditOperation, Matrix,
    MatrixAssembly, MatrixCell, Part, PartDefinition, PartKind, PartModel, PartOutline, Pose2,
    ProjectDoc, Side, Vec2,
};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::{
    NumericField, SelectField, TOUCH_CLS, TextCommitField,
};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonVariant},
    callout::{Callout, CalloutVariant},
    item::{Item, ItemGroup, ItemSize},
};
use dioxus::prelude::*;
use js_sys::{Date, Function, Reflect};
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};

#[derive(Clone, Debug, PartialEq)]
struct AssemblyDraft {
    base: Option<AssemblyDefinition>,
    value: AssemblyDefinition,
    assets: Vec<Asset>,
    document_id: String,
    session_epoch: boardstudio_application::SessionEpoch,
    scope: Option<Scope>,
}

fn derived_identity(root: &str, id: &str) -> bool {
    id == root
        || id
            .strip_prefix(&format!("{root}-"))
            .is_some_and(|suffix| suffix.parse::<u64>().is_ok())
}

fn saved_assembly_for_submission<'a>(
    document: &'a ProjectDoc,
    id: &str,
    created: bool,
) -> Option<&'a AssemblyDefinition> {
    if created {
        document
            .assemblies
            .iter()
            .rev()
            .find(|assembly| derived_identity(id, &assembly.id))
    } else {
        document
            .assemblies
            .iter()
            .find(|assembly| assembly.id == id)
    }
}

fn placed_parts_for_submission(document: &ProjectDoc, board_id: &str, seed: &str) -> Vec<String> {
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return Vec::new();
    };
    let placement_of = |part: &Part| {
        part.properties
            .as_ref()
            .and_then(|properties| properties.get("assemblyId"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    };
    let Some(placement) = document
        .parts
        .iter()
        .rev()
        .filter(|part| board.part_ids.contains(&part.id))
        .find_map(|part| placement_of(part).filter(|id| derived_identity(seed, id)))
    else {
        return Vec::new();
    };
    document
        .parts
        .iter()
        .filter(|part| {
            board.part_ids.contains(&part.id) && placement_of(part).as_ref() == Some(&placement)
        })
        .map(|part| part.id.clone())
        .collect()
}

fn assembly_save_resolver(draft: AssemblyDraft, definitions: Vec<PartDefinition>) -> EditResolver {
    replace_document_resolver("parts-assembly-editor", move |accepted, replacement| {
        if accepted.document.id != draft.document_id {
            return Err("The assembly's project changed.".into());
        }
        let mut draft = draft.clone();
        if draft.base.is_none() {
            let root = draft.value.id.clone();
            let mut suffix = 0_u64;
            while accepted
                .document
                .assemblies
                .iter()
                .any(|assembly| assembly.id == draft.value.id)
            {
                suffix += 1;
                draft.value.id = format!("{root}-{suffix}");
            }
        }
        *replacement = saved_document(&accepted.document, &draft, &definitions)?;
        Ok(vec![draft.value.id])
    })
}

fn assembly_place_resolver(
    scope: Scope,
    proposal: ProjectDoc,
    assets: Vec<Asset>,
    seed: String,
    part_ids: Vec<String>,
) -> EditResolver {
    replace_document_resolver("parts-assembly-place", move |accepted, replacement| {
        if accepted.document.id != scope.document_id {
            return Err("The assembly's project changed.".into());
        }
        let mut placement = seed.clone();
        let mut suffix = 0_u64;
        while accepted
            .document
            .parts
            .iter()
            .any(|part| part.id.starts_with(&format!("{placement}/")))
            || accepted.document.definitions.iter().any(|definition| {
                definition
                    .id
                    .starts_with(&format!("{placement}/definition/"))
            })
        {
            suffix += 1;
            placement = format!("{seed}-{suffix}");
        }
        let prefix = format!("{seed}/");
        let remap = |id: &str| {
            id.strip_prefix(&prefix)
                .map(|tail| format!("{placement}/{tail}"))
                .unwrap_or_else(|| id.to_owned())
        };
        let mut prepared = proposal.clone();
        for definition in prepared
            .definitions
            .iter_mut()
            .filter(|definition| definition.id.starts_with(&format!("{seed}/definition/")))
        {
            definition.id = remap(&definition.id);
        }
        for part in prepared
            .parts
            .iter_mut()
            .filter(|part| part_ids.contains(&part.id))
        {
            part.id = remap(&part.id);
            part.definition_id = remap(&part.definition_id);
            if let Some(properties) = part.properties.as_mut() {
                properties.insert(
                    "assemblyId".into(),
                    serde_json::Value::String(placement.clone()),
                );
            }
        }
        let ids = part_ids.iter().map(|id| remap(id)).collect::<Vec<_>>();
        *replacement = rebase_assembly_placement(
            &accepted.document,
            &prepared,
            &assets,
            &scope.board_id,
            &placement,
            &ids,
        )?;
        let mut targets = ids;
        targets.push(scope.board_id.clone());
        Ok(targets)
    })
}

fn assembly_matrix_resolver(
    scope: Scope,
    matrix_id: String,
    assembly: AssemblyDefinition,
    prepared: Vec<PartDefinition>,
    seed: String,
) -> EditResolver {
    EditResolver::new(
        "parts-assembly-matrix-apply",
        move |accepted: &AcceptedSnapshot| {
            if accepted.document.id != scope.document_id {
                return Resolution::Retire("The assembly's project changed.".into());
            }
            let Some(matrix) = accepted.document.matrices.iter().find(|matrix| {
                matrix.id == matrix_id && matrix.board_id.as_deref() == Some(&scope.board_id)
            }) else {
                return Resolution::Retire("The selected matrix is no longer on this PCB.".into());
            };
            let mut nonce = seed.clone();
            let mut suffix = 0_u64;
            while accepted.document.definitions.iter().any(|definition| {
                definition
                    .id
                    .starts_with(&format!("{matrix_id}/assembly-{nonce}/definition/"))
            }) {
                suffix += 1;
                nonce = format!("{seed}-{suffix}");
            }
            // The prepared definitions are immutable recipe snapshots. Rebind each member
            // to its template, while letting the helper derive cells from the latest matrix.
            let mut recipe = assembly.clone();
            for (member, definition) in recipe.members.iter_mut().zip(&prepared) {
                member.definition_id = Some(definition.id.clone());
            }
            match matrix_with_assembly(matrix, &recipe, &prepared, &accepted.document, &nonce) {
                Ok((matrix, mut definitions)) => {
                    for (definition, template) in definitions.iter_mut().zip(&prepared) {
                        let id = definition.id.clone();
                        *definition = template.clone();
                        definition.id = id;
                    }
                    replacement_commit(
                        EditOperation::SetMatrix {
                            matrix,
                            definitions: Some(definitions),
                        },
                        vec![matrix_id.clone()],
                    )
                }
                Err(reason) => Resolution::Retire(reason),
            }
        },
    )
}

/// Saved reusable assemblies are project data; editor fields stay local until one
/// accepted document edit commits them through the existing Session path.
#[component]
pub fn SavedAssembliesEditor(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    definitions: Vec<PartDefinition>,
    preset_definitions: Vec<PartDefinition>,
    selected_context: Signal<Option<super::super::objects::ScopedTreeContext>>,
    on_place: EventHandler<super::super::objects::MatrixPlacementSource>,
    on_board_placed: EventHandler<()>,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let workspace = use_context::<super::super::WorkspaceState>().0;
    let runtime_version = use_context::<Signal<u64>>();
    let _ = runtime_version();
    let assembly_selection = use_context::<super::PartsAssemblySelection>().0;
    let assembly_orientation = use_context::<super::PartsAssemblyOrientation>().0;
    let has_preset_definitions = !preset_definitions.is_empty();
    let mut editing = use_signal(|| None::<AssemblyDraft>);
    let mut editor_generation = use_signal(|| 0_u64);
    let preparing_apply = use_signal(|| false);
    let preparing_place = use_signal(|| false);
    let mut feedback = use_signal(|| None::<String>);

    let open_existing = {
        let snapshot = snapshot.clone();
        let scope = scope.clone();
        move |assembly: AssemblyDefinition| {
            feedback.set(None);
            editor_generation += 1;
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
                editor_generation += 1;
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
    let create_action = EventHandler::new(new_assembly);
    let action_slot = try_consume_context::<super::AssemblyCreateAction>();
    let action_owner = dioxus::dioxus_core::current_scope_id();
    use_effect(use_reactive((&create_action,), move |(action,)| {
        if let Some(super::AssemblyCreateAction(mut slot)) = action_slot {
            slot.set(Some((action_owner, action)));
        }
    }));
    use_drop(move || {
        if let Some(super::AssemblyCreateAction(mut slot)) = action_slot {
            let owned = slot
                .peek()
                .as_ref()
                .is_some_and(|(owner, _)| *owner == action_owner);
            if owned {
                slot.set(None);
            }
        }
    });
    let customize_preset = {
        let runtime = runtime.clone();
        let snapshot = snapshot.clone();
        let scope = scope.clone();
        move |_| {
            if workspace() != Workspace::Parts {
                return;
            }
            let Some(preset) = assembly_selection.peek().as_ref().copied() else {
                feedback.set(Some("Select a key assembly before customizing it.".into()));
                return;
            };
            let current_model = runtime.model();
            let Some(current) = current_model.accepted.as_ref() else {
                feedback.set(Some("The accepted project is unavailable.".into()));
                return;
            };
            if current_model.lifecycle != Lifecycle::Ready
                || current_model.durability
                    != (Durability::Saved {
                        revision: current.document.revision,
                    })
                || runtime.scope() != scope
                || current.token != snapshot.token
                || current.document.id != snapshot.document.id
                || current.document.revision != snapshot.document.revision
                || current.session_epoch != snapshot.session_epoch
            {
                feedback.set(Some(
                    "The project or Parts scope changed. Reopen the preset before customizing it."
                        .into(),
                ));
                return;
            }
            let id = match next_id(&current.document) {
                Ok(id) => id,
                Err(error) => {
                    feedback.set(Some(error));
                    return;
                }
            };
            let recipe = match customization_recipe(
                preset,
                &preset_definitions,
                assembly_orientation.peek().to_owned(),
            ) {
                Ok(recipe) => recipe,
                Err(error) => {
                    feedback.set(Some(error));
                    return;
                }
            };
            let value = crate::parts_assembly_preset_draft::from_recipe(
                id,
                name(preset).to_uppercase(),
                recipe,
            );
            feedback.set(None);
            editor_generation += 1;
            editing.set(Some(AssemblyDraft {
                base: None,
                value,
                assets: Vec::new(),
                document_id: current.document.id.clone(),
                session_epoch: current.session_epoch,
                scope: scope.clone(),
            }));
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
                editor_generation += 1;
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

    let apply_to_matrix = EventHandler::new({
        let runtime = runtime.clone();
        let scope = scope.clone();
        let snapshot = snapshot.clone();
        let selected_context = selected_context;
        let definitions = definitions.clone();
        let feedback = feedback;
        move |assembly| {
            begin_apply_assembly_to_matrix(
                runtime.clone(),
                workspace,
                scope.clone(),
                snapshot.clone(),
                selected_context,
                definitions.clone(),
                assembly,
                preparing_apply,
                editor_generation,
                feedback,
            );
        }
    });

    let place_on_board = EventHandler::new({
        let runtime = runtime.clone();
        let scope = scope.clone();
        let snapshot = snapshot.clone();
        let definitions = definitions.clone();
        let preparing = preparing_place;
        let feedback = feedback;
        move |(draft, origin_x, origin_y): (AssemblyDraft, f64, f64)| {
            begin_place_assembly_on_board(
                runtime.clone(),
                workspace,
                scope.clone(),
                snapshot.clone(),
                definitions.clone(),
                draft,
                origin_x,
                origin_y,
                preparing,
                selected_context,
                editing,
                on_board_placed,
                editor_generation,
                feedback,
            );
        }
    });

    let save = {
        let runtime = runtime.clone();
        let scope = scope.clone();
        let definitions = definitions.clone();
        move |_| {
            if workspace() != Workspace::Parts {
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
            match runtime.commit(
                "parts-assembly-editor",
                assembly_save_resolver(draft.clone(), definitions.clone()),
            ) {
                Ok(accepted) => {
                    let id = &draft.value.id;
                    if let Some(saved) =
                        saved_assembly_for_submission(&accepted.document, id, draft.base.is_none())
                    {
                        editing.with_mut(|current| {
                            if let Some(current) =
                                current.as_mut().filter(|current| &current.value.id == id)
                            {
                                current.base = Some(saved.clone());
                                current.value.id = saved.id.clone();
                                current
                                    .assets
                                    .retain(|asset| !accepted.document.assets.contains(asset));
                            }
                        });
                    }
                    feedback.set(None);
                }
                Err(message) => feedback.set(Some(message)),
            }
        }
    };

    let saved_assemblies = snapshot.document.assemblies.clone();
    let draft = editing.read().clone();
    let draft_key = draft
        .as_ref()
        .map(|draft| draft.value.id.clone())
        .unwrap_or_default();
    rsx! {
        section { class: "grid gap-2", "aria-label": "Saved assemblies",
            h2 { "Assemblies" }
            if action_slot.is_none() {
                Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, onclick: move |event| create_action.call(event), "New assembly" }
            }
            if assembly_selection().is_some() {
                Button {
                    variant: ButtonVariant::Secondary,
                    class: TOUCH_CLS,
                    disabled: !has_preset_definitions,
                    onclick: customize_preset,
                    "Customize 3D assembly"
                }
            }
            if saved_assemblies.is_empty() {
                div { role: "status", Callout { variant: CalloutVariant::Info, "No saved assemblies yet." } }
            } else {
                div { class: "grid gap-px", role: "list", "aria-label": "Saved assemblies",
                    ItemGroup {
                        for assembly in saved_assemblies {
                            { let mut open_existing = open_existing.clone(); let mut duplicate = duplicate.clone();
                              let existing_assembly = assembly.clone(); let duplicate_assembly = assembly.clone();
                              rsx! {
                                Item { size: ItemSize::Xs, key: "{assembly.id}",
                                    Button { variant: ButtonVariant::Ghost,
                                        class: "flex-1 justify-start whitespace-normal text-left {TOUCH_CLS}",
                                        onclick: move |_| open_existing(existing_assembly.clone()), "{assembly.name}" }
                                    Button { variant: ButtonVariant::Secondary,
                                        class: TOUCH_CLS,
                                        aria_label: "Duplicate {assembly.name}", onclick: move |_| duplicate(duplicate_assembly.clone()), "Duplicate" }
                                }
                              }
                            }
                        }
                    }
                }
            }
            if let Some(message) = feedback.read().as_ref() {
                div { role: "status", Callout { variant: CalloutVariant::Info, "{message}" } }
            }
            if let Some(draft) = draft {
                AssemblyDraftFields {
                    key: "{draft_key}",
                    snapshot: snapshot.clone(),
                    draft,
                    definitions: definitions.clone(),
                    place_pending: preparing_place(),
                    matrix_pending: preparing_apply(),
                    selected_context,
                    on_place,
                    on_place_board: place_on_board,
                    on_apply: apply_to_matrix,
                    on_change: move |updated: AssemblyDraft| { editing.set(Some(updated)); feedback.set(None); },
                    on_save: save,
                    on_close: move |_| { editor_generation += 1; editing.set(None); feedback.set(None); },
                }
            }
        }
    }
}

mod fields;
mod placement;
use fields::AssemblyDraftFields;
use placement::{begin_apply_assembly_to_matrix, begin_place_assembly_on_board};

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
    X(f64),
    Y(f64),
    Rotation(f64),
    Parameters(Option<std::collections::BTreeMap<String, serde_json::Value>>),
    ModelMode(bool, Vec<PartModel>),
    AddModel(PartModel),
    RemoveModel(usize),
    ModelAsset(usize, String),
    ModelVector(usize, ModelVector, ModelAxis, f64),
    RemoveMember,
}

impl MemberPatch {
    fn apply(self, member: &mut AssemblyMember) {
        match self {
            Self::Definition(id) => {
                member.definition_id = (!id.is_empty()).then_some(id);
                member.parameters = None;
                member.model_mode = Some(boardstudio_model::model::AssemblyModelMode::Defaults);
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
                if value.is_finite() {
                    member.pose.at.x = value;
                }
            }
            Self::Y(value) => {
                if value.is_finite() {
                    member.pose.at.y = value;
                }
            }
            Self::Rotation(value) => {
                if value.is_finite() {
                    member.pose.rotation = value;
                }
            }
            Self::Parameters(parameters) => member.parameters = parameters,
            Self::ModelMode(custom, models) => {
                member.model_mode = Some(if custom {
                    boardstudio_model::model::AssemblyModelMode::Custom
                } else {
                    boardstudio_model::model::AssemblyModelMode::Defaults
                });
                member.models = models;
            }
            Self::AddModel(model) => {
                member.model_mode = Some(boardstudio_model::model::AssemblyModelMode::Custom);
                member.models.push(model);
            }
            Self::RemoveModel(index) => {
                member.model_mode = Some(boardstudio_model::model::AssemblyModelMode::Custom);
                if index < member.models.len() {
                    member.models.remove(index);
                }
            }
            Self::ModelAsset(index, asset_id) => {
                if let Some(model) = member.models.get_mut(index) {
                    model.asset_id = asset_id;
                }
            }
            Self::ModelVector(index, field, axis, value) => {
                if !valid_transform_value(field, value) {
                    return;
                }
                if let Some(model) = member.models.get_mut(index) {
                    set_model_axis(model, field, axis, value);
                }
            }
            Self::RemoveMember => { /* handled by update_member's assembly-level branch */ }
        }
    }
}

#[derive(Clone, PartialEq)]
struct ModelOption {
    id: String,
    name: String,
}

fn component_option_rows(
    options: &[ModelOption],
    selected_id: Option<&str>,
) -> Vec<(String, String)> {
    let has_selected = selected_id.is_some_and(|id| options.iter().any(|option| option.id == id));
    let mut rows = Vec::with_capacity(options.len() + 2);
    rows.push((String::new(), "Visual model only".to_owned()));
    rows.extend(
        options
            .iter()
            .map(|option| (option.id.clone(), option.name.clone())),
    );
    if let Some(selected_id) = selected_id.filter(|_| !has_selected) {
        rows.push((
            selected_id.to_owned(),
            "Saved component unavailable".to_owned(),
        ));
    }
    rows
}

fn model_asset_option_rows(options: &[ModelOption], selected_id: &str) -> Vec<(String, String)> {
    let mut rows = Vec::with_capacity(options.len() + 1);
    rows.extend(
        options
            .iter()
            .map(|option| (option.id.clone(), option.name.clone())),
    );
    if !options.iter().any(|option| option.id == selected_id) {
        rows.push((
            selected_id.to_owned(),
            "Saved model asset unavailable".to_owned(),
        ));
    }
    rows
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

fn assembly_preview_recipe(
    assembly: &AssemblyDefinition,
    definitions: &[PartDefinition],
    assets: &[Asset],
) -> Vec<crate::parts_preview::PartsPreviewRecipeMember> {
    assembly
        .members
        .iter()
        .map(|member| {
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
                Some(boardstudio_model::model::AssemblyModelMode::Custom)
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

fn assembly_owner_current(
    runtime: &crate::runtime::Runtime,
    draft: &AssemblyDraft,
    workspace: Workspace,
) -> bool {
    let guard = crate::parts_custom_definition::PartsOwnerGuard::document(
        draft.scope.clone(),
        draft.session_epoch,
        draft.document_id.clone(),
    );
    runtime.model().accepted.as_ref().is_some_and(|accepted| {
        guard.is_current(accepted, runtime.scope(), None, None, None, Some(workspace))
    })
}

fn unique_asset_id(root: &str, document_assets: &[Asset], draft_assets: &[Asset]) -> String {
    crate::parts_custom_definition::unique_suffixed_id_capped(root, |id| {
        document_assets
            .iter()
            .chain(draft_assets)
            .any(|asset| asset.id == id)
    })
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
    validate_members(&draft.value, accepted, &draft.assets)?;
    let current = accepted
        .assemblies
        .iter()
        .find(|assembly| assembly.id == draft.value.id);
    match (&draft.base, current) {
        (Some(_), Some(_)) => {}
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
    if let Some(existing) = document
        .assemblies
        .iter_mut()
        .find(|existing| existing.id == assembly.id)
    {
        *existing = assembly;
    } else {
        document.assemblies.push(assembly);
    }
    Ok(document)
}

fn next_id(document: &ProjectDoc) -> Result<String, String> {
    let base = browser_uuid().unwrap_or_else(|| format!("{:x}", Date::now().max(0.0) as u64));
    crate::parts_custom_definition::unique_suffixed_id_capped(&format!("assembly-{base}"), |id| {
        document.assemblies.iter().any(|assembly| assembly.id == id)
    })
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
    crate::parts_custom_definition::unique_suffixed_id_capped(
        &format!("component-{}", &root[..root.len().min(8)]),
        |id| assembly.members.iter().any(|member| member.id == id),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixPresetId {
    MxSolder,
    MxHotswap,
    ChocSolder,
    ChocHotswap,
    MxRgb,
    ChocRgb,
    MxHotswapRgb,
    ChocHotswapRgb,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SwitchOrientation {
    #[default]
    South,
    North,
}

#[derive(Clone, Copy)]
pub struct Preset {
    pub id: MatrixPresetId,
    pub name: &'static str,
    pub definition_id: &'static str,
    family: &'static str,
    hotswap: bool,
    led: bool,
}

pub const PRESETS: [Preset; 8] = [
    Preset {
        id: MatrixPresetId::MxSolder,
        name: "MX Solder",
        definition_id: "generator:ceoloide/switch_mx",
        family: "mx",
        hotswap: false,
        led: false,
    },
    Preset {
        id: MatrixPresetId::MxHotswap,
        name: "MX Hotswap",
        definition_id: "generator:ceoloide/switch_mx",
        family: "mx",
        hotswap: true,
        led: false,
    },
    Preset {
        id: MatrixPresetId::ChocSolder,
        name: "Choc V1 Solder",
        definition_id: "generator:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: false,
        led: false,
    },
    Preset {
        id: MatrixPresetId::ChocHotswap,
        name: "Choc V1 Hotswap",
        definition_id: "generator:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: true,
        led: false,
    },
    Preset {
        id: MatrixPresetId::MxRgb,
        name: "MX RGB",
        definition_id: "generator:ceoloide/switch_mx",
        family: "mx",
        hotswap: false,
        led: true,
    },
    Preset {
        id: MatrixPresetId::ChocRgb,
        name: "Choc V1 RGB",
        definition_id: "generator:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: false,
        led: true,
    },
    Preset {
        id: MatrixPresetId::MxHotswapRgb,
        name: "MX Hotswap RGB",
        definition_id: "generator:ceoloide/switch_mx",
        family: "mx",
        hotswap: true,
        led: true,
    },
    Preset {
        id: MatrixPresetId::ChocHotswapRgb,
        name: "Choc V1 Hotswap RGB",
        definition_id: "generator:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: true,
        led: true,
    },
];

pub fn name(id: MatrixPresetId) -> &'static str {
    preset(id).name
}

fn preset(id: MatrixPresetId) -> &'static Preset {
    PRESETS
        .iter()
        .find(|preset| preset.id == id)
        .expect("all preset ids are listed")
}

pub async fn resolve(
    id: MatrixPresetId,
    entries: &[CatalogEntry],
    reversible: bool,
    orientation: SwitchOrientation,
    preview_definition: Option<PartDefinition>,
) -> Result<Vec<crate::parts_preview::PartsPreviewRecipeMember>, String> {
    let preset = preset(id);
    let mut members = recipe_members(
        preset,
        |source| {
            entries
                .iter()
                .find(|entry| {
                    entry
                        .definition
                        .generator
                        .as_ref()
                        .is_some_and(|generator| generator.source == source)
                })
                .map(|entry| (*entry.definition).clone())
        },
        reversible,
        orientation,
        preview_definition,
    )?;
    for member in &mut members {
        if let Some(generator) = member.definition.generator.as_mut() {
            generator
                .parameters
                .extend(member.generator_parameters.clone());
        }
        member.definition =
            super::catalogue::normalize_generator_definition(member.definition.clone()).await?;
    }
    Ok(members)
}

/// Resolve the library recipe used to start a customizable assembly. It uses
/// the same preset member construction as preview, but the reference creates a
/// single-sided assembly from library definitions even in reversible layouts.
pub fn customization_recipe(
    id: MatrixPresetId,
    definitions: &[PartDefinition],
    orientation: SwitchOrientation,
) -> Result<Vec<crate::parts_preview::PartsPreviewRecipeMember>, String> {
    recipe_members(
        preset(id),
        |source| {
            definitions
                .iter()
                .find(|definition| {
                    definition
                        .generator
                        .as_ref()
                        .is_some_and(|generator| generator.source == source)
                })
                .cloned()
        },
        false,
        orientation,
        None,
    )
}

fn recipe_members(
    preset: &Preset,
    mut find_definition: impl FnMut(&str) -> Option<PartDefinition>,
    reversible: bool,
    orientation: SwitchOrientation,
    preview_definition: Option<PartDefinition>,
) -> Result<Vec<crate::parts_preview::PartsPreviewRecipeMember>, String> {
    let main_source = preset
        .definition_id
        .strip_prefix("generator:")
        .unwrap_or(preset.definition_id);
    let main = find_definition(main_source)
        .ok_or_else(|| format!("Missing switch footprint: {main_source}"))?;
    let main = preview_definition
        .filter(|definition| definition.id == main.id)
        .unwrap_or(main);
    let mut members = vec![member(
        "switch",
        &main,
        Vec2 { x: 0.0, y: 0.0 },
        0.0,
        Side::Front,
        switch_parameters(preset, reversible),
    )];
    if let Some(diode) = find_definition("ceoloide/diode_tht_sod123") {
        members.push(member(
            "diode",
            &diode,
            Vec2 { x: 7.4, y: -1.5 },
            90.0,
            Side::Back,
            serde_json::from_value(serde_json::json!({
                "side": "B", "reversible": reversible, "include_tht": false
            }))
            .expect("fixed diode parameters are valid"),
        ));
    }
    if preset.led {
        let led = find_definition("ceoloide/led_sk6812mini-e")
            .ok_or_else(|| "Missing SK6812 MINI-E footprint".to_owned())?;
        members.push(member(
            "led",
            &led,
            Vec2 {
                x: 0.0,
                y: if preset.family == "choc" { -4.7 } else { -4.75 },
            },
            180.0,
            Side::Back,
            serde_json::from_value(serde_json::json!({
                "side": "B", "reverse_mount": true, "reversible": reversible
            }))
            .expect("fixed LED parameters are valid"),
        ));
    }
    for member in &mut members {
        if orientation == SwitchOrientation::North {
            member.at.x = -member.at.x;
            member.at.y = -member.at.y;
            member.rotation = (member.rotation + 180.0) % 360.0;
        }
    }
    Ok(members)
}

fn switch_parameters(
    preset: &Preset,
    reversible: bool,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    let mut parameters: std::collections::BTreeMap<String, serde_json::Value> =
        serde_json::from_value(serde_json::json!({
            "hotswap": preset.hotswap,
            "solder": !preset.hotswap,
            "reversible": reversible,
            "side": "B",
            "include_keycap": true
        }))
        .expect("fixed switch parameters are valid");
    if preset.family == "choc" {
        parameters.extend(
            serde_json::from_value::<std::collections::BTreeMap<String, serde_json::Value>>(
                serde_json::json!({
                    "choc_v1_support": true,
                    "choc_v2_support": false,
                    "include_choc_v1_led_cutout_marks": true
                }),
            )
            .expect("fixed Choc parameters are valid"),
        );
    }
    parameters
}

fn member(
    id: &str,
    definition: &PartDefinition,
    at: Vec2,
    rotation: f64,
    side: Side,
    generator_parameters: std::collections::BTreeMap<String, serde_json::Value>,
) -> crate::parts_preview::PartsPreviewRecipeMember {
    crate::parts_preview::PartsPreviewRecipeMember {
        id: id.to_owned(),
        definition: definition.clone(),
        assets: Vec::new(),
        at,
        rotation,
        side,
        generator_parameters,
    }
}

/// Applies the saved-assembly recipe to every cell while retaining matrix
/// geometry and enabled state, matching the pinned `matrixWithAssembly` helper.
/// Definitions receive operation-specific snapshot identities so Core can
/// commit recipe edits without changing already-placed part snapshots.
pub fn matrix_with_assembly(
    matrix: &Matrix,
    assembly: &AssemblyDefinition,
    catalogue_definitions: &[PartDefinition],
    document: &ProjectDoc,
    snapshot_nonce: &str,
) -> Result<(Matrix, Vec<PartDefinition>), String> {
    let Some(primary) = assembly.members.first() else {
        return Err("Name the assembly and add at least one member".into());
    };
    if primary.definition_id.is_none() {
        return Err(
            "The first assembly member must have a component definition to apply it to a matrix."
                .into(),
        );
    }
    if primary.pose.at.x != 0.0 || primary.pose.at.y != 0.0 || primary.side != Side::Front {
        return Err(
            "For matrix placement, keep the first member at the origin on the front".into(),
        );
    }
    validate_members(assembly, document, &[])?;
    let nonce = safe_identity(snapshot_nonce);
    let definitions = assembly
        .members
        .iter()
        .map(|member| {
            let mut definition = member_definition(member, document, catalogue_definitions, &[])?;
            definition.id = format!("{}/assembly-{nonce}/definition/{}", matrix.id, member.id);
            Ok(definition)
        })
        .collect::<Result<Vec<_>, String>>()?;

    let primary_rotation = primary.pose.rotation;
    let angle = -primary_rotation.to_radians();
    let (sin, cos) = angle.sin_cos();
    let assemblies = assembly
        .members
        .iter()
        .skip(1)
        .enumerate()
        .map(|(index, member)| MatrixAssembly {
            id: member.id.clone(),
            definition_id: definitions[index + 1].id.clone(),
            offset: Vec2 {
                x: member.pose.at.x * cos - member.pose.at.y * sin,
                y: member.pose.at.x * sin + member.pose.at.y * cos,
            },
            rotation: Some(member.pose.rotation - primary_rotation),
            side: Some(member.side.clone()),
        })
        .collect::<Vec<_>>();
    let mut cells = Vec::with_capacity((matrix.rows * matrix.columns) as usize);
    for row in 0..matrix.rows {
        for column in 0..matrix.columns {
            let previous = matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column);
            cells.push(MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row,
                column,
                enabled: previous.is_none_or(|cell| cell.enabled),
                definition_id: Some(definitions[0].id.clone()),
                variant: previous.and_then(|cell| cell.variant.clone()),
                offset: previous.and_then(|cell| cell.offset),
                rotation: Some(
                    previous.and_then(|cell| cell.rotation).unwrap_or(0.0) + primary_rotation,
                ),
                assemblies: assemblies.clone(),
                assemblies_local: Some(true),
            });
        }
    }
    let mut matrix = matrix.clone();
    matrix.definition_id = definitions[0].id.clone();
    matrix.cells = cells;
    Ok((matrix, definitions))
}

/// Reproduces the React editor's selected-board placement as one accepted
/// project proposal. Each assembly member gets an immutable definition/part
/// snapshot; later recipe edits therefore cannot move existing placements.
pub fn document_with_assembly(
    document: &ProjectDoc,
    assembly: &AssemblyDefinition,
    source_definitions: &[PartDefinition],
    draft_assets: &[Asset],
    board_id: &str,
    origin: Vec2,
    placement_id: &str,
) -> Result<(ProjectDoc, Vec<String>), String> {
    if assembly.members.is_empty() {
        return Err("Add at least one component before placing this assembly.".into());
    }
    if !origin.x.is_finite() || !origin.y.is_finite() {
        return Err("Assembly placement coordinates must be finite numbers.".into());
    }
    let Some(board_index) = document
        .boards
        .iter()
        .position(|board| board.id == board_id)
    else {
        return Err("The selected board is no longer available.".into());
    };
    validate_members(assembly, document, draft_assets)?;
    let mut next = document.clone();
    merge_assets(&mut next, draft_assets)?;
    let mut part_ids = Vec::with_capacity(assembly.members.len());
    for member in &assembly.members {
        let mut definition = member_definition(member, document, source_definitions, draft_assets)?;
        definition.id = format!("{placement_id}/definition/{}", member.id);
        let part_id = format!("{placement_id}/{}", member.id);
        if next
            .definitions
            .iter()
            .any(|existing| existing.id == definition.id)
            || next.parts.iter().any(|existing| existing.id == part_id)
        {
            return Err("The assembly placement identity is already in use.".into());
        }
        let reference = format!("{} {}", assembly.name, member.id);
        if next
            .parts
            .iter()
            .any(|existing| existing.reference == reference)
        {
            return Err(format!(
                "The component reference '{reference}' is already in use."
            ));
        }
        let mut properties = std::collections::BTreeMap::new();
        properties.insert(
            "assemblyId".into(),
            serde_json::Value::String(placement_id.to_owned()),
        );
        properties.insert(
            "visualOnly".into(),
            serde_json::Value::Bool(member.definition_id.is_none()),
        );
        next.definitions.push(definition.clone());
        next.parts.push(Part {
            keycap: None,
            outline: member.definition_id.is_none().then_some(PartOutline {
                excluded: true,
                ..Default::default()
            }),
            id: part_id.clone(),
            definition_id: definition.id,
            reference,
            pose: boardstudio_model::model::Pose2 {
                at: Vec2 {
                    x: origin.x + member.pose.at.x,
                    y: origin.y + member.pose.at.y,
                },
                rotation: member.pose.rotation,
            },
            side: member.side.clone(),
            anchor: None,
            locked: None,
            properties: Some(properties),
            generator_parameters: None,
        });
        part_ids.push(part_id);
    }
    let board = &mut next.boards[board_index];
    board.part_ids.extend(part_ids.iter().cloned());
    Ok((next, part_ids))
}

/// Retain unrelated accepted changes made while definition normalization was
/// in flight, and attach only this operation's assembly snapshots and assets.
pub fn rebase_assembly_placement(
    latest: &ProjectDoc,
    proposal: &ProjectDoc,
    draft_assets: &[Asset],
    board_id: &str,
    placement_id: &str,
    part_ids: &[String],
) -> Result<ProjectDoc, String> {
    let mut next = latest.clone();
    merge_assets(&mut next, draft_assets)?;

    let definition_prefix = format!("{placement_id}/definition/");
    for definition in proposal
        .definitions
        .iter()
        .filter(|definition| definition.id.starts_with(&definition_prefix))
    {
        if next
            .definitions
            .iter()
            .any(|existing| existing.id == definition.id)
        {
            return Err("The assembly placement definition identity is already in use.".into());
        }
        next.definitions.push(definition.clone());
    }
    for part_id in part_ids {
        let Some(part) = proposal.parts.iter().find(|part| part.id == *part_id) else {
            return Err("The assembly placement proposal is incomplete.".into());
        };
        if next.parts.iter().any(|existing| existing.id == *part_id)
            || next
                .parts
                .iter()
                .any(|existing| existing.reference == part.reference)
        {
            return Err("A placed assembly component identity is already in use.".into());
        }
        next.parts.push(part.clone());
    }
    let Some(board) = next.boards.iter_mut().find(|board| board.id == board_id) else {
        return Err("The selected board is no longer available.".into());
    };
    for part_id in part_ids {
        if !board.part_ids.contains(part_id) {
            board.part_ids.push(part_id.clone());
        }
    }
    Ok(next)
}

pub(super) fn merge_assets(next: &mut ProjectDoc, assets: &[Asset]) -> Result<(), String> {
    for asset in assets {
        if let Some(existing) = next.assets.iter().find(|existing| existing.id == asset.id) {
            if existing != asset {
                return Err(format!(
                    "The model asset identity '{}' conflicts with the accepted project.",
                    asset.id
                ));
            }
        } else {
            next.assets.push(asset.clone());
        }
    }

    Ok(())
}

fn asset_exists(document: &ProjectDoc, assets: &[Asset], id: &str) -> bool {
    document
        .assets
        .iter()
        .chain(assets)
        .any(|asset| asset.id == id)
        || crate::bundled_models::bundled_model(id).is_some()
        || id.starts_with("unresolved-model:")
}

pub(super) fn validate_members(
    assembly: &AssemblyDefinition,
    document: &ProjectDoc,
    assets: &[Asset],
) -> Result<(), String> {
    let mut ids = std::collections::BTreeSet::new();
    for member in &assembly.members {
        if member.id.trim().is_empty() || !ids.insert(&member.id) {
            return Err("Assembly members must have unique, non-empty identities.".into());
        }
        if ![member.pose.at.x, member.pose.at.y, member.pose.rotation]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err("Assembly member positions and angles must be finite.".into());
        }
        for model in &member.models {
            let scale = model.scale;
            let coordinates = [model.offset, model.rotation, scale];
            if !coordinates
                .into_iter()
                .flat_map(|point| [point.x, point.y, point.z])
                .all(f64::is_finite)
                || [scale.x, scale.y, scale.z]
                    .into_iter()
                    .any(|value| value <= 0.0)
            {
                return Err("Model positions and rotations must be finite, and model scales must be positive.".into());
            }
            if !asset_exists(document, assets, &model.asset_id) {
                return Err(format!(
                    "The model asset '{}' is not available in this project.",
                    model.asset_id
                ));
            }
        }
    }
    Ok(())
}

fn member_definition(
    member: &AssemblyMember,
    document: &ProjectDoc,
    catalogue: &[PartDefinition],
    assets: &[Asset],
) -> Result<PartDefinition, String> {
    let mut definition = match member.definition_id.as_deref() {
        Some(id) => document
            .definitions
            .iter()
            .chain(catalogue)
            .find(|definition| definition.id == id)
            .cloned()
            .ok_or_else(|| format!("Missing component definition: {id}"))?,
        None => model_only_definition(member),
    };
    if let (Some(generator), Some(parameters)) = (&mut definition.generator, &member.parameters) {
        generator.parameters.extend(parameters.clone());
    }
    if member.model_mode == Some(AssemblyModelMode::Custom)
        || (member.model_mode.is_none() && !member.models.is_empty())
    {
        definition.models = Some(member.models.clone());
    }
    for model in definition.models.as_deref().unwrap_or_default() {
        if !asset_exists(document, assets, &model.asset_id) {
            return Err(format!(
                "The model asset '{}' is not available in this project.",
                model.asset_id
            ));
        }
    }
    Ok(definition)
}

pub(super) fn model_only_definition(member: &AssemblyMember) -> PartDefinition {
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

fn safe_identity(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}
