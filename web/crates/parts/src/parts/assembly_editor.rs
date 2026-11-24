//! Parts-owned saved assembly list and the first authoring slice of F4.6.
use crate::parts_custom_definition::replacement_commit;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope,
};
use boardstudio_model::model::{
    AssemblyDefinition, AssemblyMember, Asset, EditOperation, Part, PartDefinition, PartModel,
    Pose2, ProjectDoc, Side, Vec2,
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
use js_sys::Date;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
struct AssemblyDraft {
    base: Option<AssemblyDefinition>,
    value: AssemblyDefinition,
    assets: Vec<Asset>,
    document_id: String,
    session_epoch: boardstudio_application::SessionEpoch,
    scope: Option<Scope>,
}

impl AssemblyDraft {
    fn open(
        base: Option<AssemblyDefinition>,
        value: AssemblyDefinition,
        snapshot: &AcceptedSnapshot,
        scope: Option<Scope>,
    ) -> Self {
        Self {
            base,
            value,
            assets: Vec::new(),
            document_id: snapshot.document.id.clone(),
            session_epoch: snapshot.session_epoch,
            scope,
        }
    }
}

/// The first `seed`, `seed-1`, `seed-2`, … identity not already taken.
fn unique_seed(seed: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut id = seed.to_owned();
    let mut suffix = 0_u64;
    while taken(&id) {
        suffix += 1;
        id = format!("{seed}-{suffix}");
    }
    id
}

fn assembly_save_resolver(draft: AssemblyDraft, definitions: Vec<PartDefinition>) -> EditResolver {
    EditResolver::new("parts-assembly-editor", move |accepted| {
        let document = &accepted.document;
        if document.id != draft.document_id || accepted.session_epoch != draft.session_epoch {
            return Resolution::Retire("The assembly's project changed.".into());
        }
        let mut assembly = draft.value.clone();
        let existing = |id: &str| document.assemblies.iter().find(|item| item.id == id);
        if draft.base.is_none() {
            assembly.id = unique_seed(&assembly.id.clone(), |id| existing(id).is_some());
        } else if existing(&assembly.id).is_none() {
            return Resolution::Retire(
                "This saved assembly changed while the editor was open. Reopen it before saving."
                    .into(),
            );
        }
        if assembly.name.trim().is_empty() || assembly.members.is_empty() {
            return Resolution::Retire("Name the assembly and add at least one member".into());
        }
        if let Err(reason) = validate_members(&assembly, document, &draft.assets) {
            return Resolution::Retire(reason);
        }
        let definitions: Vec<_> = assembly
            .members
            .iter()
            .filter_map(|member| member.definition_id.as_deref())
            .filter(|id| {
                !document
                    .definitions
                    .iter()
                    .any(|definition| definition.id == *id)
            })
            .filter_map(|id| {
                definitions
                    .iter()
                    .find(|definition| definition.id == id)
                    .cloned()
            })
            .collect();
        if existing(&assembly.id) == Some(&assembly)
            && definitions.is_empty()
            && draft
                .assets
                .iter()
                .all(|asset| document.assets.contains(asset))
        {
            return Resolution::Unchanged;
        }
        Resolution::submit(
            vec![assembly.id.clone()],
            EditOperation::UpsertAssembly {
                assembly: Box::new(assembly),
                definitions,
                assets: draft.assets.clone(),
            },
        )
    })
}

fn assembly_place_resolver(
    scope: Scope,
    definitions: Vec<PartDefinition>,
    parts: Vec<Part>,
    assets: Vec<Asset>,
    seed: String,
) -> EditResolver {
    EditResolver::new("parts-assembly-place", move |accepted| {
        let document = &accepted.document;
        if document.id != scope.document_id || accepted.session_epoch != scope.session_epoch {
            return Resolution::Retire("The assembly's project changed.".into());
        }
        let placement_id = unique_seed(&seed, |candidate| {
            document
                .parts
                .iter()
                .any(|part| part.id.starts_with(&format!("{candidate}/")))
                || document.definitions.iter().any(|definition| {
                    definition
                        .id
                        .starts_with(&format!("{candidate}/definition/"))
                })
        });
        let prefix = format!("{seed}/");
        let remap = |id: &str| {
            id.strip_prefix(&prefix)
                .map(|tail| format!("{placement_id}/{tail}"))
                .unwrap_or_else(|| id.to_owned())
        };
        let mut definitions = definitions.clone();
        for definition in &mut definitions {
            definition.id = remap(&definition.id);
        }
        let mut parts = parts.clone();
        for part in &mut parts {
            part.id = remap(&part.id);
            part.definition_id = remap(&part.definition_id);
        }
        let mut targets: Vec<_> = parts.iter().map(|part| part.id.clone()).collect();
        targets.push(scope.board_id.clone());
        Resolution::submit(
            targets,
            EditOperation::PlaceAssembly {
                board_id: scope.board_id.clone(),
                placement_id,
                definitions,
                parts,
                assets: assets.clone(),
            },
        )
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
            let nonce = unique_seed(&seed, |candidate| {
                accepted.document.definitions.iter().any(|definition| {
                    definition
                        .id
                        .starts_with(&format!("{matrix_id}/assembly-{candidate}/definition/"))
                })
            });
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
            editing.set(Some(AssemblyDraft::open(
                Some(assembly.clone()),
                assembly,
                &snapshot,
                scope.clone(),
            )));
        }
    };
    let new_assembly = {
        let snapshot = snapshot.clone();
        let scope = scope.clone();
        move |_| match next_id(&snapshot.document) {
            Ok(id) => {
                feedback.set(None);
                editor_generation += 1;
                editing.set(Some(AssemblyDraft::open(
                    None,
                    AssemblyDefinition {
                        id,
                        name: "New assembly".into(),
                        members: Vec::new(),
                    },
                    &snapshot,
                    scope.clone(),
                )));
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
        let scope = scope.clone();
        move |_| {
            if workspace() != Workspace::Parts {
                return;
            }
            let Some(preset) = assembly_selection.peek().as_ref().copied() else {
                feedback.set(Some("Select a key assembly before customizing it.".into()));
                return;
            };
            // Customizing only opens a local draft from the live accepted
            // project; the save resolver revalidates it on commit.
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
            editing.set(Some(AssemblyDraft::open(
                None,
                value,
                current,
                scope.clone(),
            )));
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
                editing.set(Some(AssemblyDraft::open(
                    None,
                    value,
                    &snapshot,
                    scope.clone(),
                )));
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
            // The commit is synchronous: a newly created assembly is the one
            // whose id was not present in the accepted document beforehand
            // (its resolver may suffix a colliding id).
            let known_ids: std::collections::BTreeSet<&str> = current
                .document
                .assemblies
                .iter()
                .map(|assembly| assembly.id.as_str())
                .collect();
            match runtime.commit(
                "parts-assembly-editor",
                assembly_save_resolver(draft.clone(), definitions.clone()),
            ) {
                Ok(accepted) => {
                    let id = &draft.value.id;
                    let saved = if draft.base.is_some() {
                        accepted
                            .document
                            .assemblies
                            .iter()
                            .find(|assembly| &assembly.id == id)
                    } else {
                        accepted
                            .document
                            .assemblies
                            .iter()
                            .find(|assembly| !known_ids.contains(assembly.id.as_str()))
                    };
                    if let Some(saved) = saved {
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
mod recipes;
use fields::AssemblyDraftFields;
use placement::{begin_apply_assembly_to_matrix, begin_place_assembly_on_board};
use recipes::validate_members;
pub use recipes::{
    MatrixPresetId, PRESETS, Preset, SwitchOrientation, customization_recipe, matrix_with_assembly,
    name, resolve,
};

fn next_id(document: &ProjectDoc) -> Result<String, String> {
    let base = crate::runtime::new_project_id()
        .unwrap_or_else(|_| format!("{:x}", Date::now().max(0.0) as u64));
    crate::parts_custom_definition::unique_suffixed_id_capped(&format!("assembly-{base}"), |id| {
        document.assemblies.iter().any(|assembly| assembly.id == id)
    })
    .ok_or_else(|| "Could not allocate a unique assembly identity.".into())
}
