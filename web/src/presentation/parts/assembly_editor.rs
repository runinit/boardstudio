//! Parts-owned saved assembly list and the first authoring slice of F4.6.
use boardstudio_application::{AcceptedSnapshot, Event, Scope, TerminalOutcome};
use boardstudio_core::model::{
    AssemblyDefinition, AssemblyMember, EditCommand, EditOperation, EditPhase, PartDefinition,
    Pose2, ProjectDoc, Side, Vec2,
};
use dioxus::prelude::*;
use js_sys::{Date, Function, Reflect};
use std::{collections::BTreeSet, rc::Rc};
use wasm_bindgen::{JsCast, JsValue};

#[derive(Clone, Debug, PartialEq)]
struct AssemblyDraft {
    base: Option<AssemblyDefinition>,
    value: AssemblyDefinition,
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
    draft: AssemblyDraft,
    definitions: Vec<PartDefinition>,
    pending: bool,
    on_change: EventHandler<AssemblyDraft>,
    on_save: EventHandler<MouseEvent>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    let value = use_signal(|| draft.value.clone());
    let render_value = value();
    let member_definitions = definitions
        .iter()
        .filter(|definition| {
            !matches!(&definition.kind, boardstudio_core::model::PartKind::Utility)
        })
        .cloned()
        .collect::<Vec<_>>();
    rsx! {
        section { class: "m1-parts-assembly-editor", "aria-label": "Assembly editor",
            h3 { "Assembly editor" }
            label { class: "m1-parts-assembly-field", "Name"
                { let name_draft = draft.clone(); let name_change = on_change.clone();
                  rsx! { input { value: "{render_value.name}", disabled: pending, oninput: move |event| { let mut value = value; value.with_mut(|value| value.name = event.value()); name_change.call(AssemblyDraft { value: value(), ..name_draft.clone() }); } } }
                }
            }
            for member in render_value.members.clone() {
                { let definition_id = member.id.clone(); let side_id = member.id.clone(); let x_id = member.id.clone(); let y_id = member.id.clone(); let rotation_id = member.id.clone(); let remove_id = member.id.clone();
                  let definition_draft = draft.clone(); let side_draft = draft.clone(); let x_draft = draft.clone(); let y_draft = draft.clone(); let rotation_draft = draft.clone(); let remove_draft = draft.clone();
                  let definition_change = on_change.clone(); let side_change = on_change.clone(); let x_change = on_change.clone(); let y_change = on_change.clone(); let rotation_change = on_change.clone(); let remove_change = on_change.clone();
                  rsx! {
                  fieldset { class: "m1-parts-assembly-member", key: "{member.id}",
                    legend { "{member.id}" }
                    label { class: "m1-parts-assembly-field", "Component"
                        select {
                            value: member.definition_id.as_deref().unwrap_or(""),
                            disabled: pending,
                            onchange: move |event| update_member(value, definition_draft.clone(), definition_change, definition_id.clone(), MemberPatch::Definition(event.value())),
                            option { value: "", "Visual model only" }
                            for definition in member_definitions.iter() {
                                option { value: "{definition.id}", "{definition.name}" }
                            }
                        }
                    }
                    label { class: "m1-parts-assembly-field", "Side"
                        select {
                            value: if matches!(&member.side, Side::Back) { "back" } else { "front" },
                            disabled: pending,
                            onchange: move |event| update_member(value, side_draft.clone(), side_change, side_id.clone(), MemberPatch::Side(event.value() == "back")),
                            option { value: "front", "Front" }
                            option { value: "back", "Back" }
                        }
                    }
                    div { class: "m1-parts-assembly-transform",
                        label { "X (mm)" input { r#type: "number", step: "0.1", value: "{member.pose.at.x}", disabled: pending, oninput: move |event| update_member(value, x_draft.clone(), x_change, x_id.clone(), MemberPatch::X(event.value())) } }
                        label { "Y (mm)" input { r#type: "number", step: "0.1", value: "{member.pose.at.y}", disabled: pending, oninput: move |event| update_member(value, y_draft.clone(), y_change, y_id.clone(), MemberPatch::Y(event.value())) } }
                        label { "Angle (°)" input { r#type: "number", step: "1", value: "{member.pose.rotation}", disabled: pending, oninput: move |event| update_member(value, rotation_draft.clone(), rotation_change, rotation_id.clone(), MemberPatch::Rotation(event.value())) } }
                    }
                    button { r#type: "button", disabled: pending, onclick: move |_| { let mut value = value; value.with_mut(|value| value.members.retain(|candidate| candidate.id != remove_id)); remove_change.call(AssemblyDraft { value: value(), ..remove_draft.clone() }); }, "Remove component" }
                  }
                  }}
            }
            { let add_draft = draft.clone(); let add_change = on_change.clone();
            rsx! { button { r#type: "button", disabled: pending, onclick: move |_| {
                let mut value = value;
                if let Some(id) = member_id(&value()) {
                    value.with_mut(|value| value.members.push(AssemblyMember {
                        parameters: None,
                        model_mode: Some(boardstudio_core::model::AssemblyModelMode::Defaults),
                        id,
                        definition_id: None,
                        pose: Pose2 { at: Vec2 { x: 0.0, y: 0.0 }, rotation: 0.0 },
                        side: Side::Front,
                        models: Vec::new(),
                    }));
                    add_change.call(AssemblyDraft { value: value(), ..add_draft.clone() });
                }
            }, "Add component" }
            }
            }
            div { class: "m1-parts-assembly-actions",
                button { r#type: "button", disabled: pending, onclick: on_save, "Save assembly" }
                button { r#type: "button", disabled: pending, onclick: on_close, "Close editor" }
            }
        }
    }
}

fn update_member(
    mut value: Signal<AssemblyDefinition>,
    draft: AssemblyDraft,
    on_change: EventHandler<AssemblyDraft>,
    member_id: String,
    patch: MemberPatch,
) {
    let mut updated = false;
    value.with_mut(|assembly| {
        if let Some(member) = assembly
            .members
            .iter_mut()
            .find(|member| member.id == member_id)
        {
            patch.apply(member);
            updated = true;
        }
    });
    if updated {
        on_change.call(AssemblyDraft {
            value: value(),
            ..draft
        });
    }
}

#[derive(Clone)]
enum MemberPatch {
    Definition(String),
    Side(bool),
    X(String),
    Y(String),
    Rotation(String),
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
            Self::Side(back) => member.side = if back { Side::Back } else { Side::Front },
            Self::X(value) => {
                if let Ok(value) = value.parse::<f64>() {
                    member.pose.at.x = value;
                }
            }
            Self::Y(value) => {
                if let Ok(value) = value.parse::<f64>() {
                    member.pose.at.y = value;
                }
            }
            Self::Rotation(value) => {
                if let Ok(value) = value.parse::<f64>() {
                    member.pose.rotation = value;
                }
            }
        }
    }
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
