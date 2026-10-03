//! Parts-owned retained Ergogen settings drafts and their accepted edit boundary.
use super::{GeneratorDraftStore, GeneratorPreviewDraft};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Event, OperationId, Scope, TerminalOutcome};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, Net, PartDefinition, Pin};
use dioxus::prelude::*;
use serde_json::Value;
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GeneratorOwner {
    pub(super) scope: Option<Scope>,
    pub(super) selection: Option<(Option<Scope>, String)>,
    pub(super) session_epoch: boardstudio_application::SessionEpoch,
    pub(super) document_id: String,
    pub(super) definition_id: String,
    pub(super) base_definition: PartDefinition,
    pub(super) source: String,
    pub(super) base_parameters: BTreeMap<String, Value>,
    pub(super) scope_generation: u64,
    pub(super) selection_generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum GeneratorPreviewStatus {
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

#[derive(Clone)]
struct PendingApply {
    owner: GeneratorOwner,
    candidate: PartDefinition,
    outcome: crate::operation_outcomes::OutcomeSlot,
}

#[derive(Clone, Debug, PartialEq)]
struct ScopedFeedback {
    owner: GeneratorOwner,
    message: String,
}

pub(super) fn owner_is_current(
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
            generator.source == owner.source && generator.parameters == owner.base_parameters
        }),
        None => true,
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
        base_parameters: generator.parameters.clone(),
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
            if parameter_group(key, &kind, &definition.kind) == "3D model placement" {
                return None;
            }
            let value = generator
                .parameters
                .get(key)
                .cloned()
                .or_else(|| spec.get("value").cloned())
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
    part_kind: &boardstudio_core::model::PartKind,
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
    ) || (matches!(part_kind, boardstudio_core::model::PartKind::Utility)
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

fn apply_input(
    definition: &PartDefinition,
    schema: &BTreeMap<String, Value>,
    edits: &BTreeMap<String, Value>,
) -> Result<PartDefinition, String> {
    let Some(base) = definition.generator.as_ref() else {
        return Err("The selected part is no longer generator-backed.".into());
    };
    let mut generator = base.clone();
    for (key, spec) in schema {
        let Some(kind) = spec.get("type").and_then(Value::as_str) else {
            continue;
        };
        let raw = edits
            .get(key)
            .or_else(|| generator.parameters.get(key))
            .or_else(|| spec.get("value"));
        let Some(raw) = raw else {
            continue;
        };
        let value = match kind {
            "number" => {
                let number = match raw {
                    Value::Number(number) => number.as_f64(),
                    Value::String(text) if !text.trim().is_empty() => text.trim().parse().ok(),
                    _ => None,
                }
                .filter(|number: &f64| number.is_finite())
                .ok_or_else(|| format!("{} must be a finite number.", key.replace('_', " ")))?;
                serde_json::Number::from_f64(number)
                    .map(Value::Number)
                    .ok_or_else(|| format!("{} must be a finite number.", key.replace('_', " ")))?
            }
            "boolean" => raw
                .as_bool()
                .map(Value::Bool)
                .ok_or_else(|| format!("{} must be true or false.", key.replace('_', " ")))?,
            "string" | "net" => raw
                .as_str()
                .map(|value| Value::String(value.to_owned()))
                .ok_or_else(|| format!("{} must be text.", key.replace('_', " ")))?,
            "array" | "object" | "anchor" => {
                let value = if let Some(text) = raw.as_str() {
                    serde_json::from_str::<Value>(text).map_err(|_| {
                        format!("{} must contain valid JSON.", key.replace('_', " "))
                    })?
                } else {
                    raw.clone()
                };
                let valid = if kind == "array" {
                    value.is_array()
                } else {
                    value.is_object()
                };
                if !valid {
                    return Err(format!(
                        "{} must be {}.",
                        key.replace('_', " "),
                        if kind == "array" {
                            "a JSON array"
                        } else {
                            "a JSON object"
                        }
                    ));
                }
                value
            }
            _ => continue,
        };
        generator.parameters.insert(key.clone(), value);
    }
    let mut candidate = definition.clone();
    candidate.generator = Some(generator);
    Ok(candidate)
}

pub(super) async fn prepare_generator_candidate(
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

fn remap_terminal_nets(
    snapshot: &AcceptedSnapshot,
    original: &PartDefinition,
    candidate: &PartDefinition,
) -> Result<Vec<Net>, String> {
    let instances = snapshot
        .document
        .parts
        .iter()
        .filter(|part| part.definition_id == candidate.id)
        .collect::<Vec<_>>();
    let mut assignments = Vec::<(String, Vec<String>, Vec<String>, String)>::new();
    for part in instances {
        for (terminal, old_pads) in &original.terminals {
            let net_ids = snapshot
                .document
                .nets
                .iter()
                .filter(|net| {
                    net.pins
                        .iter()
                        .any(|pin| pin.part_id == part.id && old_pads.contains(&pin.pad_id))
                })
                .map(|net| net.id.clone())
                .collect::<BTreeSet<_>>();
            if net_ids.is_empty() {
                continue;
            }
            let new_pads = candidate.terminals.get(terminal);
            if net_ids.len() > 1 || new_pads.is_none_or(Vec::is_empty) {
                return Err(format!(
                    "Cannot apply these generator settings: assigned {terminal} terminal pads would be lost or are split across nets."
                ));
            }
            assignments.push((
                part.id.clone(),
                old_pads.clone(),
                new_pads.expect("checked above").clone(),
                net_ids.into_iter().next().expect("one assigned net"),
            ));
        }
    }
    let mut old_pads_by_part = BTreeMap::<String, BTreeSet<String>>::new();
    for (part_id, old_pads, _, _) in &assignments {
        old_pads_by_part
            .entry(part_id.clone())
            .or_default()
            .extend(old_pads.iter().cloned());
    }
    let retained = |pin: &Pin| {
        !old_pads_by_part
            .get(&pin.part_id)
            .is_some_and(|pads| pads.contains(&pin.pad_id))
    };
    let mut destination_nets = BTreeMap::<(String, String), BTreeSet<String>>::new();
    for net in &snapshot.document.nets {
        for pin in net.pins.iter().filter(|pin| retained(pin)) {
            destination_nets
                .entry((pin.part_id.clone(), pin.pad_id.clone()))
                .or_default()
                .insert(net.id.clone());
        }
    }
    for (part_id, _, new_pads, net_id) in &assignments {
        for pad_id in new_pads {
            let key = (part_id.clone(), pad_id.clone());
            if destination_nets
                .get(&key)
                .is_some_and(|owners| owners.iter().any(|owner| owner != net_id))
            {
                return Err(format!(
                    "Cannot apply these generator settings: destination pad {pad_id} would be assigned to multiple nets."
                ));
            }
            destination_nets.insert(key, BTreeSet::from([net_id.clone()]));
        }
    }
    let mut nets = snapshot.document.nets.clone();
    for net in &mut nets {
        net.pins.retain(&retained);
    }
    for (part_id, _, new_pads, net_id) in assignments {
        let Some(net) = nets.iter_mut().find(|net| net.id == net_id) else {
            return Err(
                "A terminal net changed while generator settings were being applied.".into(),
            );
        };
        for pad_id in new_pads {
            if !net
                .pins
                .iter()
                .any(|pin| pin.part_id == part_id && pin.pad_id == pad_id)
            {
                net.pins.push(Pin {
                    part_id: part_id.clone(),
                    pad_id,
                });
            }
        }
    }
    Ok(nets)
}

fn prepare_generator_edit(
    current: &AcceptedSnapshot,
    owner: &GeneratorOwner,
    view: &GeneratorView,
    candidate: PartDefinition,
    operation_id: OperationId,
) -> Result<Event, String> {
    if view.workspace != "Parts"
        || view.scope != owner.scope
        || view.selection != owner.selection
        || view.scope_generation != owner.scope_generation
        || view.selection_generation != owner.selection_generation
        || current.session_epoch != owner.session_epoch
        || current.document.id != owner.document_id
        || candidate.id != owner.definition_id
        || candidate
            .generator
            .as_ref()
            .map(|generator| generator.source.as_str())
            != Some(owner.source.as_str())
    {
        return Err("The Parts project, generator selection, or view changed before Apply.".into());
    }
    let original = current
        .document
        .definitions
        .iter()
        .find(|definition| definition.id == owner.definition_id)
        .cloned()
        .unwrap_or_else(|| owner.base_definition.clone());
    if original.generator.as_ref().is_none_or(|generator| {
        generator.source != owner.source || generator.parameters != owner.base_parameters
    }) {
        return Err(
            "The selected generator definition changed while these settings were open.".into(),
        );
    }
    let nets = remap_terminal_nets(current, &original, &candidate)?;
    let mut document = current.document.as_ref().clone();
    if let Some(existing) = document
        .definitions
        .iter_mut()
        .find(|definition| definition.id == candidate.id)
    {
        *existing = candidate.clone();
    } else {
        document.definitions.push(candidate.clone());
    }
    document.nets = nets;
    let mut target_ids = vec![candidate.id.clone()];
    target_ids.extend(
        document
            .parts
            .iter()
            .filter(|part| part.definition_id == candidate.id)
            .map(|part| part.id.clone()),
    );
    Ok(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: current.document.revision,
            transaction_id: format!("parts-generator-settings-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids,
            operation: EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        },
    })
}

#[derive(Clone)]
struct GeneratorView {
    scope: Option<Scope>,
    selection: Option<(Option<Scope>, String)>,
    scope_generation: u64,
    selection_generation: u64,
    workspace: &'static str,
}

/// Dynamic settings form. Each field remains a local JSON draft until Apply; every
/// candidate preview uses the same retained generator renderer as the Parts canvas.
#[component]
pub(super) fn GeneratorSettingsEditor(
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
    let version = use_context::<Signal<u64>>();
    let generator_source = definition
        .generator
        .as_ref()
        .map(|generator| generator.source.clone())
        .unwrap_or_default();
    let schema = use_resource(use_reactive((&generator_source,), |(source,)| async move {
        super::catalogue::ergogen_parameter_schema(&source).await
    }));
    let schema_result = schema.read().clone();
    let edits = use_signal(BTreeMap::<String, Value>::new);
    let mut feedback = use_signal(|| None::<ScopedFeedback>);
    let pending_apply = use_signal(|| None::<PendingApply>);
    let sequence = use_hook(|| Rc::new(Cell::new(0_u64)));
    let owner = make_owner(
        &snapshot,
        scope.clone(),
        selected(),
        &definition,
        scope_generation(),
        selection_generation(),
    );
    let definition_identity = (
        definition.id.clone(),
        generator_source.clone(),
        definition
            .generator
            .as_ref()
            .map(|generator| generator.parameters.clone()),
        scope.clone(),
        selected(),
    );
    use_effect(use_reactive((&definition_identity,), {
        let mut edits = edits;
        let mut store = store;
        move |_| {
            edits.set(BTreeMap::new());
            store.set(None);
        }
    }));

    use_effect(use_reactive((&version(),), {
        let runtime = runtime.clone();
        let mut pending_apply = pending_apply;
        let mut feedback = feedback;
        let mut store = store;
        move |_| {
            let Some(pending) = pending_apply.read().clone() else {
                return;
            };
            let Some(outcome) = pending.outcome.borrow().clone() else {
                return;
            };
            pending_apply.set(None);
            if !owner_context_is_current(
                &pending.owner,
                &runtime,
                selected,
                scope_generation(),
                selection_generation(),
                workspace(),
            ) {
                return;
            }
            match outcome {
                TerminalOutcome::Completed => {
                    let current = runtime.model().accepted;
                    let accepted = current.as_ref().and_then(|snapshot| {
                        snapshot.document.definitions.iter().find(|definition| {
                            definition.id == pending.candidate.id
                                && definition.generator == pending.candidate.generator
                        })
                    });
                    if accepted.is_some() {
                        store.set(None);
                        feedback.set(None);
                    }
                }
                TerminalOutcome::Rejected(reason)
                | TerminalOutcome::PersistenceFailed(reason)
                | TerminalOutcome::BlockedByRecovery(reason)
                | TerminalOutcome::ExecutorFailed(reason) => feedback.set(Some(ScopedFeedback {
                    owner: pending.owner.clone(),
                    message: reason,
                })),
                TerminalOutcome::Cancelled => feedback.set(Some(ScopedFeedback {
                    owner: pending.owner.clone(),
                    message: "Generator settings were cancelled.".into(),
                })),
                TerminalOutcome::Closed | TerminalOutcome::Superseded => {}
            }
        }
    }));

    let Some(owner) = owner else {
        return rsx! {};
    };
    let schema = match schema_result {
        Some(Ok(schema)) => schema,
        Some(Err(error)) => {
            return rsx! { p { class: "m1-parts-load-error", role: "alert", "Generator settings could not be loaded: {error}" } };
        }
        None => {
            return rsx! { p { class: "m1-parts-loading", role: "status", "Loading generator settings…" } };
        }
    };
    let entries = parameters(&definition, &schema);
    let mut changed = edits;
    let callback_owner = owner.clone();
    let callback_runtime = runtime.clone();
    let change_parameter = use_callback(move |(key, value): (String, Value)| {
        let mut next = changed();
        next.insert(key, value);
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
        let schema = schema.clone();
        let scope_generation = scope_generation;
        let selection_generation = selection_generation;
        let selected = selected;
        let workspace = workspace;
        let runtime = callback_runtime.clone();
        let mut store = store;
        let task = sequence.clone();
        let ticket = task.get().wrapping_add(1);
        task.set(ticket);
        spawn_local(async move {
            let result = prepare_generator_candidate(base, schema, next).await;
            if task.get() != ticket
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
    let active_draft = store().filter(|draft| draft.owner == owner);
    let preview_ready = active_draft.as_ref().is_some_and(|draft| {
        draft.status == GeneratorPreviewStatus::Ready && draft.definition.is_some()
    });
    let on_apply = {
        let runtime = runtime.clone();
        let mut pending_apply = pending_apply;
        let mut feedback = feedback;
        let current_owner = owner.clone();
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
            let model = runtime.model();
            let Some(current) = model.accepted.as_ref() else {
                feedback.set(Some(ScopedFeedback {
                    owner: current_owner.clone(),
                    message: "The accepted project is unavailable.".into(),
                }));
                return;
            };
            let view = GeneratorView {
                scope: runtime.scope(),
                selection: selected(),
                scope_generation: scope_generation(),
                selection_generation: selection_generation(),
                workspace: workspace(),
            };
            match prepare_generator_edit(
                current,
                &current_owner,
                &view,
                candidate.clone(),
                runtime.operation(),
            ) {
                Ok(event) => {
                    let operation_id = match &event {
                        Event::Edit { operation_id, .. } => *operation_id,
                        _ => unreachable!("generator edit helper returns Edit"),
                    };
                    let outcome = runtime.observe_operation(operation_id);
                    pending_apply.set(Some(PendingApply {
                        owner: current_owner.clone(),
                        candidate,
                        outcome,
                    }));
                    feedback.set(None);
                    runtime.submit(event);
                }
                Err(error) => feedback.set(Some(ScopedFeedback {
                    owner: current_owner.clone(),
                    message: error,
                })),
            }
        }
    };

    let groups = [
        "Footprint options",
        "Keycap dimensions",
        "Connections",
        "Advanced footprint options",
    ];
    rsx! {
        section { class: "m1-generator-settings", "aria-label": "Generator settings",
            for group in groups {
                { let group_entries = entries.iter().filter(|entry| entry.group == group).cloned().collect::<Vec<_>>();
                  rsx! {
                    if !group_entries.is_empty() {
                    details { open: group == "Footprint options" || group == "Keycap dimensions",
                        summary { "{group}" }
                        fieldset {
                            class: "m1-generator-fields",
                            "aria-label": "{group}",
                            disabled: pending_apply().is_some(),
                            for entry in group_entries {
                                GeneratorParameterField {
                                    entry: entry.clone(),
                                    value: edits().get(&entry.key).cloned().unwrap_or(entry.value.clone()),
                                on_change: EventHandler::new({ let key = entry.key.clone(); move |value| change_parameter.call((key.clone(), value)) }),
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
                    GeneratorPreviewStatus::Pending => rsx! { p { role: "status", "Generating footprint preview…" } },
                    GeneratorPreviewStatus::Ready => rsx! { p { role: "status", "Current generator preview is ready to apply." } },
                    GeneratorPreviewStatus::Failed(error) => rsx! { p { class: "m1-parts-load-error", role: "alert", "Generator preview failed: {error}" } },
                }
            }
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
                p { class: "m1-parts-load-error", role: "alert", "{message.message}" }
            }
            button {
                class: "m1-generator-apply",
                r#type: "button",
                disabled: !preview_ready || pending_apply().is_some(),
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
) -> Element {
    let key = entry.key.clone();
    let label = entry.label.clone();
    if key == "side" && matches!(value.as_str(), Some("F" | "B")) {
        let selected = value.as_str().unwrap_or("F").to_owned();
        return rsx! {
            label { class: "m1-generator-field", "{label}",
                select {
                    aria_label: "side",
                    value: "{selected}",
                    onchange: move |event| on_change.call(Value::String(event.value())),
                    option { value: "F", "Front" }
                    option { value: "B", "Back" }
                }
            }
        };
    }
    match entry.kind.as_str() {
        "boolean" => rsx! {
            label { class: "m1-generator-toggle",
                input {
                    r#type: "checkbox",
                    aria_label: "{key}",
                    checked: value.as_bool().unwrap_or(false),
                    onchange: move |event| on_change.call(Value::Bool(event.checked())),
                }
                "{label}"
            }
        },
        "array" | "object" | "anchor" => {
            let current = input_value(&value, &entry.kind);
            rsx! {
                label { class: "m1-generator-field", "{label}",
                    textarea {
                        aria_label: "{key}",
                        rows: 3,
                        spellcheck: "false",
                        value: "{current}",
                        oninput: move |event| on_change.call(Value::String(event.value())),
                    }
                }
            }
        }
        "number" => {
            let current = input_value(&value, "number");
            rsx! {
                label { class: "m1-generator-field", "{label}",
                    input {
                        r#type: "number",
                        step: "any",
                        aria_label: "{label}",
                        value: "{current}",
                        oninput: move |event| on_change.call(Value::String(event.value())),
                    }
                }
            }
        }
        _ => {
            let current = input_value(&value, &entry.kind);
            rsx! {
                label { class: "m1-generator-field", "{label}",
                    input {
                        r#type: "text",
                        aria_label: "{key}",
                        value: "{current}",
                        oninput: move |event| on_change.call(Value::String(event.value())),
                    }
                }
            }
        }
    }
}
