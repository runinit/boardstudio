//! Parts-owned saved assembly list and the first authoring slice of F4.6.
use crate::parts_custom_definition::replacement_commit;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope,
};
use boardstudio_core::model::{
    AssemblyDefinition, AssemblyMember, Asset, EditOperation, Part, PartDefinition, PartKind,
    PartModel, Pose2, ProjectDoc, Side, Vec2, Vec3,
};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
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
struct AssemblySubmission {
    target: AssemblyAction,
    ticket: EditTicket,
    view_generation: u64,
    scope: Option<Scope>,
    document_id: String,
    session_epoch: boardstudio_application::SessionEpoch,
}

#[derive(Clone)]
enum AssemblyAction {
    Assembly {
        id: String,
        created: bool,
    },
    Matrix(String),
    BoardAssembly {
        scope: Scope,
        board_id: String,
        assembly_id: String,
        placement_seed: String,
    },
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
    EditResolver::new(
        "parts-assembly-editor",
        move |accepted: &AcceptedSnapshot| {
            if accepted.document.id != draft.document_id
                || accepted.session_epoch != draft.session_epoch
            {
                return Resolution::Retire("The assembly's project changed.".into());
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
            match saved_document(&accepted.document, &draft, &definitions) {
                Ok(document) if document == *accepted.document => Resolution::Unchanged,
                Ok(document) => replacement_commit(
                    EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                    vec![draft.value.id],
                ),
                Err(reason) => Resolution::Retire(reason),
            }
        },
    )
}

fn assembly_place_resolver(
    scope: Scope,
    proposal: ProjectDoc,
    assets: Vec<Asset>,
    seed: String,
    part_ids: Vec<String>,
) -> EditResolver {
    EditResolver::new(
        "parts-assembly-place",
        move |accepted: &AcceptedSnapshot| {
            if accepted.document.id != scope.document_id
                || accepted.session_epoch != scope.session_epoch
            {
                return Resolution::Retire("The assembly's project changed.".into());
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
            match super::assembly_presets::rebase_assembly_placement(
                &accepted.document,
                &prepared,
                &assets,
                &scope.board_id,
                &placement,
                &ids,
            ) {
                Ok(document) => {
                    let mut targets = ids;
                    targets.push(scope.board_id.clone());
                    replacement_commit(
                        EditOperation::ReplaceDocument {
                            document: Box::new(document),
                        },
                        targets,
                    )
                }
                Err(reason) => Resolution::Retire(reason),
            }
        },
    )
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
            if accepted.document.id != scope.document_id
                || accepted.session_epoch != scope.session_epoch
            {
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
            match super::assembly_presets::matrix_with_assembly(
                matrix,
                &recipe,
                &prepared,
                &accepted.document,
                &nonce,
            ) {
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
    let mut pending = use_signal(Vec::<AssemblySubmission>::new);
    let preparing_apply = use_signal(|| false);
    let preparing_place = use_signal(|| false);
    let mut feedback = use_signal(|| None::<String>);

    use_effect({
        let runtime = runtime.clone();
        let mut workspace = workspace;
        let mut selected_context = selected_context;
        move || {
            let _ = runtime_version();
            let submissions = pending.peek().clone();
            let mut retained = Vec::new();
            for waiting in &submissions {
                let model = runtime.model();
                let live = editor_generation() == waiting.view_generation
                    && workspace() == "Parts"
                    && runtime.scope() == waiting.scope
                    && model.accepted.as_ref().is_some_and(|accepted| {
                        accepted.document.id == waiting.document_id
                            && accepted.session_epoch == waiting.session_epoch
                    });
                match waiting.ticket.settlement(live) {
                    Settlement::Pending => retained.push(waiting.clone()),
                    Settlement::Retired => {}
                    Settlement::Failed { message } => feedback.set(Some(message)),
                    Settlement::Landed { .. } => {
                        let Some(accepted) = model.accepted.as_ref() else {
                            continue;
                        };
                        match &waiting.target {
                            AssemblyAction::Assembly { id, created } => {
                                if let Some(saved) = saved_assembly_for_submission(&accepted.document, id, *created) {
                                    editing.with_mut(|draft| {
                                        if let Some(draft) = draft.as_mut().filter(|draft| draft.value.id == *id) {
                                            draft.base = Some(saved.clone());
                                            draft.value.id = saved.id.clone();
                                            draft.assets.retain(|asset| !accepted.document.assets.contains(asset));
                                        }
                                    });
                                }
                            }
                            AssemblyAction::Matrix(id) => {
                                if selected_context.peek().as_ref().is_some_and(|selected| matches!(&selected.context, super::super::objects::TreeContext::Matrix { matrix_id } if matrix_id == id)) {
                                    feedback.set(Some("Assembly applied to the selected matrix.".into()));
                                }
                            }
                            AssemblyAction::BoardAssembly { scope, board_id, assembly_id, placement_seed } => {
                                if !editing.peek().as_ref().is_some_and(|draft| draft.value.id == *assembly_id) { continue; }
                                let part_ids = placed_parts_for_submission(&accepted.document, board_id, placement_seed);
                                if let Some(first) = part_ids.first() {
                                    selected_context.set(super::super::objects::context_for_part(&model, first).map(|context| super::super::objects::ScopedTreeContext { scope: scope.clone(), context }));
                                    editing.set(None);
                                    on_board_placed.call(());
                                    workspace.set("Layout");
                                    runtime.submit(Event::SelectParts { operation_id: runtime.operation(), part_ids, range_part_ids: Vec::new(), mode: boardstudio_application::SelectionMode::Replace });
                                }
                            }
                        }
                    }
                }
            }
            if retained.len() != submissions.len() {
                pending.set(retained);
            }
        }
    });

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
    let customize_preset = {
        let runtime = runtime.clone();
        let workspace = workspace;
        let snapshot = snapshot.clone();
        let scope = scope.clone();
        move |_| {
            if pending.read().iter().any(|edit| {
                matches!(edit.target, AssemblyAction::Assembly { .. }) && edit.ticket.is_pending()
            }) || workspace() != "Parts"
            {
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
            let recipe = match super::assembly_presets::customization_recipe(
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
                super::assembly_presets::name(preset).to_uppercase(),
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
        let workspace = workspace;
        let scope = scope.clone();
        let snapshot = snapshot.clone();
        let selected_context = selected_context;
        let definitions = definitions.clone();
        let preparing_apply = preparing_apply;
        let pending = pending;
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
                pending,
                editor_generation,
                feedback,
            );
        }
    });

    let place_on_board = EventHandler::new({
        let runtime = runtime.clone();
        let workspace = workspace;
        let scope = scope.clone();
        let snapshot = snapshot.clone();
        let definitions = definitions.clone();
        let preparing = preparing_place;
        let pending = pending;
        let feedback = feedback;
        move |(draft, origin_x, origin_y): (AssemblyDraft, String, String)| {
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
                pending,
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
            if pending.read().iter().any(|edit| {
                matches!(edit.target, AssemblyAction::Assembly { .. }) && edit.ticket.is_pending()
            }) || workspace() != "Parts"
            {
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
            let ticket = EditTicket::begin(
                &runtime,
                "parts-assembly-editor",
                Some("assembly".into()),
                assembly_save_resolver(draft.clone(), definitions.clone()),
            );
            pending.write().push(AssemblySubmission {
                target: AssemblyAction::Assembly {
                    id: draft.value.id.clone(),
                    created: draft.base.is_none(),
                },
                ticket,
                view_generation: editor_generation(),
                scope: draft.scope.clone(),
                document_id: draft.document_id.clone(),
                session_epoch: draft.session_epoch,
            });
            feedback.set(None);
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
            button { class: "m1-parts-create-component", r#type: "button", onclick: new_assembly, "New assembly" }
            if assembly_selection().is_some() {
                button {
                    class: "m1-parts-customize-preset",
                    r#type: "button",
                    disabled: !has_preset_definitions,
                    onclick: customize_preset,
                    "Customize 3D assembly"
                }
            }
            if saved_assemblies.is_empty() {
                p { class: "m1-parts-empty", "No saved assemblies yet." }
            } else {
                div { class: "m1-parts-assembly-saved-list", role: "list", "aria-label": "Saved assemblies",
                    for assembly in saved_assemblies {
                        { let mut open_existing = open_existing.clone(); let mut duplicate = duplicate.clone();
                          let existing_assembly = assembly.clone(); let duplicate_assembly = assembly.clone();
                          rsx! {
                            div { class: "m1-parts-assembly-saved-row", key: "{assembly.id}",
                                button { r#type: "button", onclick: move |_| open_existing(existing_assembly.clone()), "{assembly.name}" }
                                button { r#type: "button", aria_label: "Duplicate {assembly.name}", onclick: move |_| duplicate(duplicate_assembly.clone()), "Duplicate" }
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
                    save_pending: pending.read().iter().any(|edit| matches!(edit.target, AssemblyAction::Assembly { .. }) && edit.ticket.is_pending()),
                    place_pending: preparing_place() || pending.read().iter().any(|edit| matches!(edit.target, AssemblyAction::BoardAssembly { .. }) && edit.ticket.is_pending()),
                    matrix_pending: preparing_apply() || pending.read().iter().any(|edit| matches!(edit.target, AssemblyAction::Matrix(_)) && edit.ticket.is_pending()),
                    selected_context,
                    on_place: on_place.clone(),
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

#[allow(clippy::too_many_arguments)]
fn begin_place_assembly_on_board(
    runtime: Rc<crate::runtime::Runtime>,
    workspace: Signal<&'static str>,
    scope: Option<Scope>,
    source: AcceptedSnapshot,
    definitions: Vec<PartDefinition>,
    draft: AssemblyDraft,
    origin_x: String,
    origin_y: String,
    mut preparing: Signal<bool>,
    mut pending: Signal<Vec<AssemblySubmission>>,
    editor_generation: Signal<u64>,
    mut feedback: Signal<Option<String>>,
) {
    let view_generation = editor_generation();
    if preparing()
        || pending.read().iter().any(|edit| {
            matches!(edit.target, AssemblyAction::BoardAssembly { .. }) && edit.ticket.is_pending()
        })
        || workspace() != "Parts"
    {
        return;
    }
    let Some(scope) = scope else {
        feedback.set(Some("Select a board before placing an assembly.".into()));
        return;
    };
    if draft.scope.as_ref() != Some(&scope)
        || draft.document_id != source.document.id
        || draft.session_epoch != source.session_epoch
    {
        feedback.set(Some(
            "The project or board scope changed while this assembly draft was open. Reopen it before placing.".into(),
        ));
        return;
    }
    let origin = match (
        origin_x.trim().parse::<f64>(),
        origin_y.trim().parse::<f64>(),
    ) {
        (Ok(x), Ok(y)) if x.is_finite() && y.is_finite() => Vec2 { x, y },
        _ => {
            feedback.set(Some(
                "Enter finite X and Y coordinates for assembly placement.".into(),
            ));
            return;
        }
    };
    let model = runtime.model();
    let Some(accepted) = model.accepted.as_ref() else {
        feedback.set(Some("The accepted project is unavailable.".into()));
        return;
    };
    if runtime.scope().as_ref() != Some(&scope)
        || accepted.document.id != source.document.id
        || accepted.session_epoch != source.session_epoch
    {
        feedback.set(Some(
            "The project changed while this assembly draft was open. Reopen it before placing."
                .into(),
        ));
        return;
    }

    let operation_id = runtime.operation();
    let placement_id = format!("assembly-{}-{}", draft.value.id, operation_id.0);
    let (mut proposal, part_ids) = match super::assembly_presets::document_with_assembly(
        &accepted.document,
        &draft.value,
        &definitions,
        &draft.assets,
        &scope.board_id,
        origin,
        &placement_id,
    ) {
        Ok(proposal) => proposal,
        Err(error) => {
            feedback.set(Some(error));
            return;
        }
    };
    let project_id = accepted.document.id.clone();
    let session_epoch = accepted.session_epoch;
    let definition_prefix = format!("{placement_id}/definition/");
    preparing.set(true);
    feedback.set(None);
    spawn_local(async move {
        let normalized = async {
            for definition in proposal
                .definitions
                .iter_mut()
                .filter(|definition| definition.id.starts_with(&definition_prefix))
            {
                if let Some(source) = definition
                    .generator
                    .as_ref()
                    .map(|generator| generator.source.clone())
                    && crate::bundled_models::is_generator_source(&source).await?
                {
                    *definition = super::normalize_matrix_definition(definition.clone()).await?;
                }
            }
            Ok::<_, String>(())
        }
        .await;
        if editor_generation() != view_generation
            || workspace() != "Parts"
            || runtime.scope().as_ref() != Some(&scope)
        {
            preparing.set(false);
            return;
        }
        preparing.set(false);
        if let Err(error) = normalized {
            feedback.set(Some(error));
            return;
        }

        let current = runtime.model();
        let Some(latest) = current.accepted.as_ref() else {
            feedback.set(Some("The accepted project is unavailable.".into()));
            return;
        };
        if workspace() != "Parts"
            || runtime.scope().as_ref() != Some(&scope)
            || latest.document.id != project_id
            || latest.session_epoch != session_epoch
        {
            feedback.set(Some(
                "The project, a component definition, or board scope changed while the assembly was being prepared. Reopen the editor and try again.".into(),
            ));
            return;
        }
        let ticket = EditTicket::begin(
            &runtime,
            "parts-assembly-place",
            Some("assembly placement".into()),
            assembly_place_resolver(
                scope.clone(),
                proposal,
                draft.assets.clone(),
                placement_id.clone(),
                part_ids,
            ),
        );
        pending.write().push(AssemblySubmission {
            target: AssemblyAction::BoardAssembly {
                scope: scope.clone(),
                board_id: scope.board_id.clone(),
                assembly_id: draft.value.id,
                placement_seed: placement_id,
            },
            ticket,
            view_generation,
            scope: Some(scope),
            document_id: project_id,
            session_epoch,
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn begin_apply_assembly_to_matrix(
    runtime: Rc<crate::runtime::Runtime>,
    workspace: Signal<&'static str>,
    scope: Option<Scope>,
    source: AcceptedSnapshot,
    selected_context: Signal<Option<super::super::objects::ScopedTreeContext>>,
    catalogue_definitions: Vec<PartDefinition>,
    assembly: AssemblyDefinition,
    mut preparing: Signal<bool>,
    mut pending: Signal<Vec<AssemblySubmission>>,
    editor_generation: Signal<u64>,
    mut feedback: Signal<Option<String>>,
) {
    let view_generation = editor_generation();
    if preparing()
        || pending.read().iter().any(|edit| {
            matches!(edit.target, AssemblyAction::Matrix(_)) && edit.ticket.is_pending()
        })
        || workspace() != "Parts"
    {
        return;
    }
    let Some(scope) = scope else {
        feedback.set(Some(
            "Open a saved board before applying an assembly to a matrix.".into(),
        ));
        return;
    };
    let Some(selected) = selected_context.peek().clone() else {
        feedback.set(Some("Select a matrix before applying an assembly.".into()));
        return;
    };
    if selected.scope != scope {
        feedback.set(Some("The selected matrix belongs to a different board. Select a matrix on the active board.".into()));
        return;
    }
    let super::super::objects::TreeContext::Matrix { matrix_id } = &selected.context else {
        feedback.set(Some("Select a matrix before applying an assembly.".into()));
        return;
    };
    let matrix_id = matrix_id.clone();
    let model = runtime.model();
    let Some(accepted) = model.accepted.as_ref() else {
        feedback.set(Some("The accepted project is unavailable.".into()));
        return;
    };
    if runtime.scope().as_ref() != Some(&scope)
        || accepted.document.id != source.document.id
        || accepted.session_epoch != source.session_epoch
    {
        feedback.set(Some(
            "The project changed while this assembly draft was open. Reopen it before applying."
                .into(),
        ));
        return;
    }
    let Some(matrix) = accepted
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .cloned()
    else {
        feedback.set(Some("The selected matrix is no longer available.".into()));
        return;
    };
    if matrix.board_id.as_deref() != Some(scope.board_id.as_str()) {
        feedback.set(Some("Select a matrix on the active board.".into()));
        return;
    }
    let accepted = accepted.clone();
    let session_epoch = scope.session_epoch;
    let document_id = scope.document_id.clone();
    let operation_id = runtime.operation();
    preparing.set(true);
    feedback.set(None);
    spawn_local(async move {
        let prepared = async {
            let (matrix, mut definitions) = super::assembly_presets::matrix_with_assembly(
                &matrix,
                &assembly,
                &catalogue_definitions,
                &accepted.document,
                &operation_id.0.to_string(),
            )?;
            for definition in &mut definitions {
                *definition = super::normalize_matrix_definition(definition.clone()).await?;
            }
            Ok::<_, String>((matrix, definitions))
        }
        .await;
        if editor_generation() != view_generation
            || workspace() != "Parts"
            || runtime.scope().as_ref() != Some(&scope)
        {
            preparing.set(false);
            return;
        }
        preparing.set(false);
        let (_matrix, definitions) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                feedback.set(Some(error));
                return;
            }
        };
        let current_model = runtime.model();
        let still_selected = selected_context.peek().as_ref() == Some(&selected);
        if workspace() != "Parts"
            || runtime.scope().as_ref() != Some(&scope)
            || !still_selected
            || !current_model.accepted.as_ref().is_some_and(|current| {
                current.session_epoch == session_epoch && current.document.id == document_id
            })
        {
            feedback.set(Some("The project or matrix selection changed while the assembly was being prepared. Reopen the editor and try again.".into()));
            return;
        }
        let ticket = EditTicket::begin(
            &runtime,
            "parts-assembly-matrix-apply",
            Some("matrix assembly".into()),
            assembly_matrix_resolver(
                scope.clone(),
                matrix_id.clone(),
                assembly,
                definitions,
                operation_id.0.to_string(),
            ),
        );
        pending.write().push(AssemblySubmission {
            target: AssemblyAction::Matrix(matrix_id),
            ticket,
            view_generation,
            scope: Some(scope),
            document_id,
            session_epoch,
        });
    });
}

#[component]
fn AssemblyDraftFields(
    snapshot: AcceptedSnapshot,
    draft: AssemblyDraft,
    definitions: Vec<PartDefinition>,
    save_pending: bool,
    place_pending: bool,
    matrix_pending: bool,
    selected_context: Signal<Option<super::super::objects::ScopedTreeContext>>,
    on_place: EventHandler<super::super::objects::MatrixPlacementSource>,
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
            !matches!(&definition.kind, boardstudio_core::model::PartKind::Utility)
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
                super::super::objects::TreeContext::Matrix { .. }
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
                button { r#type: "button", disabled: controls_disabled || save_pending, onclick: on_save, "Save assembly" }
                button { r#type: "button", disabled: controls_disabled, onclick: on_close, "Close editor" }
            }
            div { class: "m1-parts-assembly-actions",
                button {
                    r#type: "button",
                    disabled: controls_disabled || unsaved_assets,
                    onclick: move |_| on_place.call(super::super::objects::MatrixPlacementSource::Assembly {
                        assembly: placement_value.clone(),
                        definitions: placement_definitions.clone(),
                    }),
                    "Place assembly in Layout"
                }
                if matrix_selected {
                    button {
                        r#type: "button",
                        disabled: controls_disabled || matrix_pending || unsaved_assets,
                        onclick: move |_| on_apply.call(apply_value.clone()),
                        "Apply to selected matrix"
                    }
                }
            }
            fieldset { class: "m1-parts-assembly-place",
                legend { "Place assembly" }
                div { class: "m1-parts-assembly-transform",
                    label { "X (mm)"
                        input {
                            r#type: "number",
                            step: "0.1",
                            value: "{origin_x}",
                            disabled: controls_disabled,
                            oninput: move |event| origin_x.set(event.value()),
                        }
                    }
                    label { "Y (mm)"
                        input {
                            r#type: "number",
                            step: "0.1",
                            value: "{origin_y}",
                            disabled: controls_disabled,
                            oninput: move |event| origin_y.set(event.value()),
                        }
                    }
                }
                button {
                    r#type: "button",
                    disabled: controls_disabled || place_pending || draft.scope.is_none() || render_value.members.is_empty(),
                    onclick: move |_| on_place_board.call((
                        AssemblyDraft {
                            value: render_value.clone(),
                            assets: assets(),
                            ..draft.clone()
                        },
                        origin_x(),
                        origin_y(),
                    )),
                    "Place on selected board"
                }
            }
            if unsaved_assets {
                p { class: "m1-parts-empty", role: "status", "Save imported model assets before applying this assembly to a matrix." }
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
            let mut assets = assets;
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
        let on_change = on_change.clone();
        let workspace = workspace;
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
            let on_change = on_change.clone();
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
                            Some(boardstudio_core::model::AssemblyModelMode::Custom);
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
        fieldset { class: "m1-parts-assembly-member", key: "{component_id}",
            legend { "{member.id}" }
            label { class: "m1-parts-assembly-field", "Component"
                select {
                    value: member.definition_id.as_deref().unwrap_or(""),
                    disabled,
                    onchange: move |event| update_member(value, assets, definition_draft.clone(), definition_change.clone(), definition_id.clone(), MemberPatch::Definition(event.value())),
                    {component_options(&definition_options, member.definition_id.as_deref())}
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
                button { r#type: "button", disabled, onclick: edit_defaults, "Edit model defaults" }
            } else {
                p { class: "m1-parts-empty", "No component model defaults are attached." }
            }
            if is_custom {
                button { r#type: "button", disabled: disabled || options.is_empty(), onclick: move |_| {
                    if let Some(option) = options.first() {
                        update_member(value, assets, add_model_draft.clone(), on_change.clone(), add_model_id.clone(), MemberPatch::AddModel(default_model(option.id.clone())));
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
                    {model_asset_options(&asset_options, &model.asset_id)}
                }
            }
            ModelVectorFields { model: model.clone(), member_id: member_id.clone(), index, field: ModelVector::Offset, draft: offset_draft, value, assets, disabled, on_change: offset_change }
            ModelVectorFields { model: model.clone(), member_id: member_id.clone(), index, field: ModelVector::Rotation, draft: rotation_draft, value, assets, disabled, on_change: rotation_change }
            ModelVectorFields { model: model.clone(), member_id: member_id.clone(), index, field: ModelVector::Scale, draft: scale_draft, value, assets, disabled, on_change: scale_change }
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

#[derive(Clone, Copy, PartialEq)]
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

#[derive(Clone, PartialEq)]
struct ModelOption {
    id: String,
    name: String,
}

fn component_options(options: &[ModelOption], selected_id: Option<&str>) -> Element {
    let has_selected = selected_id.is_some_and(|id| options.iter().any(|option| option.id == id));
    rsx! {
        option { value: "", selected: selected_id.is_none(), "Visual model only" }
        for (index, option) in options.iter().enumerate() {
            option {
                key: "{index}-{option.id}",
                value: "{option.id}",
                selected: selected_id == Some(option.id.as_str()),
                "{option.name}"
            }
        }
        if let Some(selected_id) = selected_id.filter(|_| !has_selected) {
            option { value: "{selected_id}", selected: true, "Saved component unavailable" }
        }
    }
}

fn model_asset_options(options: &[ModelOption], selected_id: &str) -> Element {
    rsx! {
        for (index, option) in options.iter().enumerate() {
            option {
                key: "{index}-{option.id}",
                value: "{option.id}",
                selected: option.id == selected_id,
                "{option.name}"
            }
        }
        if !options.iter().any(|option| option.id == selected_id) {
            option { value: "{selected_id}", selected: true, "Saved model asset unavailable" }
        }
    }
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
            || asset_id.starts_with("unresolved-model:")
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

#[cfg(all(test, target_arch = "wasm32"))]
mod assembly_selector_tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[derive(Clone)]
    struct Probe {
        assembly: AssemblyDefinition,
        component_options: Vec<ModelOption>,
        asset_options: Vec<ModelOption>,
        component_signal: Rc<RefCell<Option<Signal<Vec<ModelOption>>>>>,
        asset_signal: Rc<RefCell<Option<Signal<Vec<ModelOption>>>>>,
    }

    #[component]
    fn persisted_assembly_fixture() -> Element {
        let probe = use_context::<Probe>();
        let component_options_signal = use_signal(Vec::<ModelOption>::new);
        let asset_options_signal = use_signal(Vec::<ModelOption>::new);
        *probe.component_signal.borrow_mut() = Some(component_options_signal);
        *probe.asset_signal.borrow_mut() = Some(asset_options_signal);
        let assembly = probe.assembly.clone();
        let member = assembly.members.first().expect("persisted member").clone();
        rsx! {
            select {
                id: "persisted-component",
                value: member.definition_id.as_deref().unwrap_or(""),
                {component_options(&component_options_signal(), member.definition_id.as_deref())}
            }
            for (index, model) in member.models.iter().enumerate() {
                select {
                    id: "persisted-model-{index}",
                    value: "{model.asset_id}",
                    {model_asset_options(&asset_options_signal(), &model.asset_id)}
                }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn persisted_member_selection_survives_dynamic_options_mount() {
        let assembly: AssemblyDefinition = serde_json::from_value(serde_json::json!({
            "id": "assembly-f46",
            "name": "F46 saved assembly",
            "members": [{
                "id": "switch",
                "definitionId": "generator:ceoloide/switch_mx",
                "modelMode": "custom",
                "parameters": { "hotswap": true },
                "pose": { "at": { "x": 4.0, "y": 0.0 }, "rotation": 0.0 },
                "side": "back",
                "models": [
                    {
                        "assetId": "bundled-model:kiswitch/SW_Hotswap_Kailh_MX.stp",
                        "offset": { "x": 0.0, "y": 0.0, "z": -1.8709399700164795 },
                        "rotation": { "x": 180.0, "y": 0.0, "z": 0.0 },
                        "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
                    },
                    {
                        "assetId": "bundled-model:kiswitch/SW_Cherry_MX_PCB.stp",
                        "offset": { "x": 0.007089999970048666, "y": 0.007089999970048666, "z": -1.3574800491333008 },
                        "rotation": { "x": 180.0, "y": 0.0, "z": 0.0 },
                        "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
                    }
                ]
            }]
        }))
        .expect("saved assembly archive shape");
        let member = &assembly.members[0];
        assert_eq!(
            member.definition_id.as_deref(),
            Some("generator:ceoloide/switch_mx")
        );
        assert_eq!(
            member.model_mode,
            Some(boardstudio_core::model::AssemblyModelMode::Custom)
        );
        assert_eq!(member.side, Side::Back);
        assert_eq!(member.pose.at.x, 4.0);

        let probe = Probe {
            assembly,
            component_options: vec![
                ModelOption {
                    id: "assembly-preset-mx-hotswap-south-left-keys-0/definition/switch".into(),
                    name: "switch mx".into(),
                },
                ModelOption {
                    id: "generator:ceoloide/switch_mx".into(),
                    name: "switch mx".into(),
                },
            ],
            asset_options: vec![
                ModelOption {
                    id: "bundled-model:kiswitch/SW_Hotswap_Kailh_MX.stp".into(),
                    name: "SW_Hotswap_Kailh_MX.stp".into(),
                },
                ModelOption {
                    id: "bundled-model:kiswitch/SW_Cherry_MX_PCB.stp".into(),
                    name: "SW_Cherry_MX_PCB.stp".into(),
                },
            ],
            component_signal: Rc::new(RefCell::new(None)),
            asset_signal: Rc::new(RefCell::new(None)),
        };
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("parts-assembly-persisted-select-test");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(persisted_assembly_fixture);
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;

        let mut component_signal = probe.component_signal.borrow().as_ref().copied().unwrap();
        component_signal.set(probe.component_options.clone());
        let mut asset_signal = probe.asset_signal.borrow().as_ref().copied().unwrap();
        asset_signal.set(probe.asset_options.clone());
        gloo_timers::future::TimeoutFuture::new(40).await;

        let selected_value = |selector: &str| {
            let select = root
                .query_selector(selector)
                .unwrap()
                .expect("persisted select");
            js_sys::Reflect::get(select.as_ref(), &JsValue::from_str("value"))
                .unwrap()
                .as_string()
                .unwrap()
        };
        assert_eq!(
            selected_value("#persisted-component"),
            "generator:ceoloide/switch_mx"
        );
        assert_eq!(
            selected_value("#persisted-model-0"),
            "bundled-model:kiswitch/SW_Hotswap_Kailh_MX.stp"
        );
        assert_eq!(
            selected_value("#persisted-model-1"),
            "bundled-model:kiswitch/SW_Cherry_MX_PCB.stp"
        );
        root.remove();
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod resolution_tests {
    use super::*;
    use boardstudio_web_runtime::runtime::project_name_test_support as support;
    use wasm_bindgen_test::*;

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("assembly-resolution", "Assembly resolution");
        document.definitions.push(serde_json::from_value(serde_json::json!({"id":"part", "name":"Part", "kind":"custom", "courtyard":[], "pads":[]})).unwrap());
        document.boards.push(serde_json::from_value(serde_json::json!({"id":"board", "name":"Board", "outlineIds":[], "partIds":[], "netIds":[], "thickness":1.6, "traces":[], "vias":[]})).unwrap());
        document.matrices.push(serde_json::from_value(serde_json::json!({"id":"matrix", "name":"Keys", "rows":1, "columns":1, "pitch":{"x":19,"y":19}, "origin":{"x":0,"y":0}, "definitionId":"part", "partIds":[], "boardId":"board", "cells":[]})).unwrap());
        document
    }
    fn assembly() -> AssemblyDefinition {
        serde_json::from_value(serde_json::json!({"id":"assembly", "name":"Assembly", "members":[{"id":"primary", "definitionId":"part", "pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front", "models":[]}]})).unwrap()
    }
    async fn release(
        runtime: &Rc<crate::runtime::Runtime>,
        release: futures_channel::oneshot::Sender<()>,
    ) {
        release.send(()).unwrap();
        gloo_timers::future::TimeoutFuture::new(30).await;
        support::run_pending(runtime).await;
    }
    fn mounted_host() -> Element {
        let runtime = use_context::<Rc<crate::runtime::Runtime>>();
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
        let workspace = use_signal(|| "Parts");
        use_context_provider(|| crate::presentation::WorkspaceState(workspace));
        let selection = use_signal(|| None);
        use_context_provider(|| super::super::PartsAssemblySelection(selection));
        let orientation = use_signal(super::super::assembly_presets::SwitchOrientation::default);
        use_context_provider(|| super::super::PartsAssemblyOrientation(orientation));
        let generation = use_signal(|| 1_u64);
        use_context_provider(|| super::super::PartsSelectionGeneration(generation));
        let activation = use_signal(|| 0_u64);
        use_context_provider(|| super::super::PartsPreviewActivation(activation));
        let theme = use_memo(|| "light");
        use_context_provider(|| crate::presentation::ResolvedTheme(theme));
        let selected_context = use_signal(|| None);
        let scope = use_signal(|| runtime.scope());
        let adapter = use_hook(|| {
            crate::presentation::SelectionAdapter::new(selected_context, scope, generation)
        });
        use_context_provider(|| adapter.clone());
        let accepted = runtime.model().accepted.unwrap();
        rsx! { SavedAssembliesEditor { snapshot: accepted.clone(), scope: runtime.scope(), definitions: accepted.document.definitions.clone(), preset_definitions: Vec::new(), selected_context, on_place: |_| {}, on_board_placed: |_| {} } }
    }

    #[wasm_bindgen_test]
    async fn mounted_board_placement_selects_new_parts_only_after_landing() {
        let runtime = support::new_runtime();
        let mut document = document();
        document.assemblies.push(assembly());
        support::open_document(&runtime, document).await;
        let dom_document = web_sys::window().unwrap().document().unwrap();
        let root = dom_document.create_element("div").unwrap();
        dom_document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(mounted_host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        let button = |label: &str| {
            let buttons = root.query_selector_all("button").unwrap();
            (0..buttons.length())
                .filter_map(|index| buttons.get(index))
                .map(|button| button.dyn_into::<web_sys::HtmlElement>().unwrap())
                .find(|button| button.text_content().as_deref() == Some(label))
                .unwrap()
        };
        button("Assembly").click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        let (mut entered, release) = support::gate_next_persist(&runtime);
        button("Place on selected board").click();
        let mut held = false;
        for _ in 0..100 {
            support::drive_pending(&runtime);
            gloo_timers::future::TimeoutFuture::new(20).await;
            if entered.try_recv().unwrap().is_some() {
                held = true;
                break;
            }
        }
        assert!(held, "{}", root.text_content().unwrap_or_default());
        assert!(runtime.model().selected_part_ids.is_empty());
        assert!(button("Place on selected board").has_attribute("disabled"));
        release.send(()).unwrap();
        gloo_timers::future::TimeoutFuture::new(40).await;
        support::run_pending(&runtime).await;
        gloo_timers::future::TimeoutFuture::new(40).await;
        let model = runtime.model();
        assert_eq!(model.selected_part_ids.len(), 1);
        assert!(
            model
                .accepted
                .unwrap()
                .document
                .parts
                .iter()
                .any(|part| model.selected_part_ids.contains(&part.id))
        );
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn assembly_followup_keeps_exact_save_and_placement_identity() {
        let original = document();
        let recipe = assembly();
        let (proposal, ids) = super::super::assembly_presets::document_with_assembly(
            &original,
            &recipe,
            &[],
            &[],
            "board",
            Vec2::default(),
            "placement",
        )
        .unwrap();
        let mut existing = proposal.clone();
        existing
            .parts
            .iter_mut()
            .find(|part| part.id == ids[0])
            .unwrap()
            .reference = "Existing placement".into();
        existing.assemblies.push(recipe.clone());
        let mut sibling = recipe.clone();
        sibling.id = "assembly-1".into();
        existing.assemblies.push(sibling.clone());
        let runtime = support::new_runtime();
        support::open_document(&runtime, existing).await;
        let accepted = runtime.model().accepted.unwrap();
        let scope = runtime.scope().unwrap();
        let mut value = recipe.clone();
        value.name = "Updated".into();
        let save = EditTicket::begin(
            &runtime,
            "save-identity-test",
            None,
            assembly_save_resolver(
                AssemblyDraft {
                    base: Some(recipe),
                    value,
                    assets: Vec::new(),
                    document_id: accepted.document.id.clone(),
                    session_epoch: accepted.session_epoch,
                    scope: Some(scope.clone()),
                },
                Vec::new(),
            ),
        );
        support::run_pending(&runtime).await;
        assert!(matches!(save.settlement(true), Settlement::Landed { .. }));
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            saved_assembly_for_submission(&accepted.document, "assembly", false)
                .unwrap()
                .id,
            "assembly"
        );
        assert_eq!(accepted.document.assemblies[1], sibling);
        let place = EditTicket::begin(
            &runtime,
            "place-identity-test",
            None,
            assembly_place_resolver(scope, proposal, Vec::new(), "placement".into(), ids.clone()),
        );
        support::run_pending(&runtime).await;
        assert!(matches!(place.settlement(true), Settlement::Landed { .. }));
        let accepted = runtime.model().accepted.unwrap();
        let selected = placed_parts_for_submission(&accepted.document, "board", "placement");
        assert_eq!(selected, vec!["placement-1/primary".to_string()]);
        assert!(accepted.document.parts.iter().any(|part| part.id == ids[0]));
    }

    #[wasm_bindgen_test]
    async fn matrix_recipe_keeps_queued_geometry_and_undo_order() {
        let runtime = support::new_runtime();
        support::open_document(&runtime, document()).await;
        let accepted = runtime.model().accepted.unwrap();
        let scope = runtime.scope().unwrap();
        let (_, prepared) = super::super::assembly_presets::matrix_with_assembly(
            &accepted.document.matrices[0],
            &assembly(),
            &[],
            &accepted.document,
            "prepared",
        )
        .unwrap();
        let (entered, gate) = support::gate_next_core_reply(&runtime);
        let first = EditTicket::begin(
            &runtime,
            "matrix-position-test",
            None,
            EditResolver::new("matrix-position-test", |accepted: &AcceptedSnapshot| {
                let mut matrix = accepted.document.matrices[0].clone();
                matrix.origin.x = 42.0;
                replacement_commit(
                    EditOperation::SetMatrix {
                        matrix,
                        definitions: None,
                    },
                    vec!["matrix".into()],
                )
            }),
        );
        support::drive_pending(&runtime);
        entered.await.unwrap();
        let second = EditTicket::begin(
            &runtime,
            "matrix-recipe-test",
            None,
            assembly_matrix_resolver(
                scope,
                "matrix".into(),
                assembly(),
                prepared,
                "recipe".into(),
            ),
        );
        release(&runtime, gate).await;
        assert!(matches!(first.settlement(true), Settlement::Landed { .. }));
        assert!(matches!(second.settlement(true), Settlement::Landed { .. }));
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(accepted.document.matrices[0].origin.x, 42.0);
        assert!(
            accepted.document.matrices[0]
                .definition_id
                .contains("assembly-recipe")
        );
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(accepted.document.matrices[0].origin.x, 42.0);
        assert_eq!(accepted.document.matrices[0].definition_id, "part");
    }
    #[wasm_bindgen_test]
    async fn assembly_save_and_place_compose_with_queued_rename() {
        let runtime = support::new_runtime();
        support::open_document(&runtime, document()).await;
        let accepted = runtime.model().accepted.unwrap();
        let scope = runtime.scope().unwrap();
        let draft = AssemblyDraft {
            base: None,
            value: assembly(),
            assets: Vec::new(),
            document_id: accepted.document.id.clone(),
            session_epoch: accepted.session_epoch,
            scope: Some(scope.clone()),
        };
        let (proposal, ids) = super::super::assembly_presets::document_with_assembly(
            &accepted.document,
            &draft.value,
            &[],
            &[],
            "board",
            Vec2 { x: 3.0, y: 5.0 },
            "placement",
        )
        .unwrap();
        let (entered, gate) = support::gate_next_core_reply(&runtime);
        let rename = EditTicket::begin(
            &runtime,
            "rename-test",
            None,
            crate::parts_definition_name::definition_name_resolver("part".into(), "Renamed".into()),
        );
        support::drive_pending(&runtime);
        entered.await.unwrap();
        let save = EditTicket::begin(
            &runtime,
            "assembly-save-test",
            None,
            assembly_save_resolver(draft, Vec::new()),
        );
        let place = EditTicket::begin(
            &runtime,
            "assembly-place-test",
            None,
            assembly_place_resolver(scope, proposal, Vec::new(), "placement".into(), ids.clone()),
        );
        assert!(matches!(place.settlement(true), Settlement::Pending));
        release(&runtime, gate).await;
        for ticket in [rename, save, place] {
            assert!(matches!(ticket.settlement(true), Settlement::Landed { .. }));
        }
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(accepted.document.definitions[0].name, "Renamed");
        assert_eq!(accepted.document.assemblies.len(), 1);
        assert!(accepted.document.boards[0].part_ids.contains(&ids[0]));
        assert_eq!(
            accepted
                .document
                .parts
                .iter()
                .find(|part| part.id == ids[0])
                .unwrap()
                .pose
                .at,
            Vec2 { x: 3.0, y: 5.0 }
        );
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        let accepted = runtime.model().accepted.unwrap();
        assert!(!accepted.document.parts.iter().any(|part| part.id == ids[0]));
        assert_eq!(accepted.document.assemblies.len(), 1);
        assert_eq!(accepted.document.definitions[0].name, "Renamed");
    }
}
