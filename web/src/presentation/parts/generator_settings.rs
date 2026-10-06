//! Parts-owned retained generator settings drafts and their accepted edit boundary.
use super::{GeneratorDraftStore, GeneratorPreviewDraft};
use crate::{presentation::model_asset_import::read_model_file, runtime::Runtime};
use boardstudio_application::{AcceptedSnapshot, Event, OperationId, Scope, TerminalOutcome};
use boardstudio_core::model::{
    Asset, EditCommand, EditOperation, EditPhase, Net, PartDefinition, Pin,
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

#[derive(Clone, Debug)]
pub(crate) struct GeneratorOwner {
    pub(super) scope: Option<Scope>,
    pub(super) selection: Option<(Option<Scope>, String)>,
    pub(super) session_epoch: boardstudio_application::SessionEpoch,
    pub(super) document_id: String,
    pub(super) definition_id: String,
    pub(super) base_definition: PartDefinition,
    pub(super) source: String,
    pub(super) generator_version: String,
    pub(super) base_parameters: BTreeMap<String, Value>,
    pub(super) project_owned_at_start: bool,
    pub(super) scope_generation: u64,
    pub(super) selection_generation: u64,
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
            && self.base_parameters == other.base_parameters
            && self.scope_generation == other.scope_generation
            && self.selection_generation == other.selection_generation
    }
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
            generator.source == owner.source
                && generator.version == owner.generator_version
                && generator.parameters == owner.base_parameters
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
        base_parameters: generator.parameters.clone(),
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
        || candidate.generator.as_ref().is_none_or(|generator| {
            generator.source != owner.source || generator.version != owner.generator_version
        })
    {
        return Err("The Parts project, generator selection, or view changed before Apply.".into());
    }
    let accepted_definition = current
        .document
        .definitions
        .iter()
        .find(|definition| definition.id == owner.definition_id)
        .cloned();
    if accepted_definition.is_none() && owner.project_owned_at_start {
        return Err("The selected project generator definition was removed.".into());
    }
    let original = accepted_definition.unwrap_or_else(|| owner.base_definition.clone());
    if original.generator.as_ref().is_none_or(|generator| {
        generator.source != owner.source
            || generator.version != owner.generator_version
            || generator.parameters != owner.base_parameters
    }) {
        return Err(
            "The selected generator definition changed while these settings were open.".into(),
        );
    }
    let candidate = rebase_generator_candidate(&owner.base_definition, &original, &candidate)?;
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

fn prepare_generator_asset_edit(
    current: &AcceptedSnapshot,
    owner: &GeneratorOwner,
    view: &GeneratorView,
    candidate: PartDefinition,
    asset: Asset,
    operation_id: OperationId,
) -> Result<Event, String> {
    let mut event = prepare_generator_edit(current, owner, view, candidate, operation_id)?;
    let Event::Edit { command, .. } = &mut event else {
        return Err("The model attachment did not produce a document edit.".into());
    };
    let EditOperation::ReplaceDocument { document } = &mut command.operation else {
        return Err("The model attachment did not produce a document replacement.".into());
    };
    if document
        .assets
        .iter()
        .any(|existing| existing.id == asset.id)
    {
        return Err("The model asset identity is already in use.".into());
    }
    document.assets.push(asset.clone());
    command.target_ids.push(asset.id);
    Ok(event)
}

fn merge_generator_field<T: Clone + PartialEq>(
    base: &T,
    latest: &T,
    candidate: &T,
    rebased: &mut T,
    label: &str,
) -> Result<(), String> {
    if candidate != base {
        if latest != base && latest != candidate {
            return Err(format!(
                "The selected generator definition's {label} changed while this draft was open."
            ));
        }
        *rebased = candidate.clone();
    }
    Ok(())
}

fn rebase_generator_candidate(
    base: &PartDefinition,
    latest: &PartDefinition,
    candidate: &PartDefinition,
) -> Result<PartDefinition, String> {
    if base.id != latest.id || candidate.id != latest.id {
        return Err("The selected generator definition changed identity.".into());
    }
    let mut rebased = latest.clone();
    merge_generator_field(
        &base.hardware_profile,
        &latest.hardware_profile,
        &candidate.hardware_profile,
        &mut rebased.hardware_profile,
        "hardware profile",
    )?;
    merge_generator_field(
        &base.input_profile,
        &latest.input_profile,
        &candidate.input_profile,
        &mut rebased.input_profile,
        "input profile",
    )?;
    merge_generator_field(
        &base.name,
        &latest.name,
        &candidate.name,
        &mut rebased.name,
        "name",
    )?;
    merge_generator_field(
        &base.kind,
        &latest.kind,
        &candidate.kind,
        &mut rebased.kind,
        "kind",
    )?;
    merge_generator_field(
        &base.keycap,
        &latest.keycap,
        &candidate.keycap,
        &mut rebased.keycap,
        "keycap",
    )?;
    merge_generator_field(
        &base.envelope_source,
        &latest.envelope_source,
        &candidate.envelope_source,
        &mut rebased.envelope_source,
        "envelope source",
    )?;
    merge_generator_field(
        &base.kicad_source,
        &latest.kicad_source,
        &candidate.kicad_source,
        &mut rebased.kicad_source,
        "kicad source",
    )?;
    merge_generator_field(
        &base.terminals,
        &latest.terminals,
        &candidate.terminals,
        &mut rebased.terminals,
        "terminals",
    )?;
    merge_generator_field(
        &base.matrix_terminals,
        &latest.matrix_terminals,
        &candidate.matrix_terminals,
        &mut rebased.matrix_terminals,
        "matrix terminals",
    )?;
    merge_generator_field(
        &base.envelope_notice,
        &latest.envelope_notice,
        &candidate.envelope_notice,
        &mut rebased.envelope_notice,
        "envelope notice",
    )?;
    merge_generator_field(
        &base.courtyard,
        &latest.courtyard,
        &candidate.courtyard,
        &mut rebased.courtyard,
        "courtyard",
    )?;
    merge_generator_field(
        &base.pads,
        &latest.pads,
        &candidate.pads,
        &mut rebased.pads,
        "pads",
    )?;
    merge_generator_field(
        &base.models,
        &latest.models,
        &candidate.models,
        &mut rebased.models,
        "models",
    )?;
    merge_generator_field(
        &base.generator,
        &latest.generator,
        &candidate.generator,
        &mut rebased.generator,
        "generator",
    )?;
    merge_generator_field(
        &base.mechanical_profile,
        &latest.mechanical_profile,
        &candidate.mechanical_profile,
        &mut rebased.mechanical_profile,
        "mechanical profile",
    )?;
    Ok(rebased)
}

#[derive(Clone)]
struct GeneratorView {
    scope: Option<Scope>,
    selection: Option<(Option<Scope>, String)>,
    scope_generation: u64,
    selection_generation: u64,
    workspace: &'static str,
}

#[cfg(test)]
type GeneratorCandidateFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<PartDefinition, String>>>>;

/// Per-mounted-editor boundary for deterministic completion ordering in browser tests.
/// The production path still calls `prepare_generator_candidate` directly.
#[cfg(test)]
#[derive(Clone)]
struct GeneratorCandidateTestProvider(
    Rc<
        dyn Fn(
            PartDefinition,
            BTreeMap<String, Value>,
            BTreeMap<String, Value>,
        ) -> GeneratorCandidateFuture,
    >,
);

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
        super::catalogue::generator_parameter_schema(&source).await
    }));
    let schema_result = schema.read().clone();
    let edits = use_signal(BTreeMap::<String, Value>::new);
    let mut feedback = use_signal(|| None::<ScopedFeedback>);
    let pending_apply = use_signal(|| None::<PendingApply>);
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
    let initial_parameters = schema
        .iter()
        .filter_map(|(key, spec)| {
            definition
                .generator
                .as_ref()
                .and_then(|generator| generator.parameters.get(key))
                .or_else(|| spec.get("value"))
                .map(|value| (key.clone(), value.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut changed = edits;
    let callback_owner = owner.clone();
    let callback_runtime = runtime.clone();
    let callback_schema = schema.clone();
    let preview_alive = alive.clone();
    #[cfg(test)]
    let candidate_provider = try_consume_context::<GeneratorCandidateTestProvider>();
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
        let task = sequence.clone();
        let alive = preview_alive.clone();
        let ticket = task.get().wrapping_add(1);
        task.set(ticket);
        #[cfg(test)]
        let candidate_provider = candidate_provider.clone();
        spawn_local(async move {
            #[cfg(test)]
            let result = if let Some(provider) = candidate_provider {
                (provider.0)(base, schema, next).await
            } else {
                prepare_generator_candidate(base, schema, next).await
            };
            #[cfg(not(test))]
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
        next.insert(key, value);
        request_preview.call(next);
    });
    // The form and its first transient preview must use the same saved/default values.
    // Owner changes also retire the previous draft and start a fresh guarded request.
    use_effect(use_reactive((&owner, &schema), move |_| {
        request_preview.call(initial_parameters.clone());
    }));
    let active_draft = store().filter(|draft| draft.owner == owner);
    let preview_ready = active_draft.as_ref().is_some_and(|draft| {
        draft.status == GeneratorPreviewStatus::Ready && draft.definition.is_some()
    });
    let model_import_runtime = runtime.clone();
    let model_import_owner = owner.clone();
    let import_generator_model = use_callback(move |(parameter, file): (String, web_sys::File)| {
        let runtime = model_import_runtime.clone();
        let owner = model_import_owner.clone();
        if model_uploading()
            || pending_apply().is_some()
            || !parameter.ends_with("3dmodel_filename")
        {
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
        let mut pending_apply = pending_apply;
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
                    generator.source != owner.source
                        || generator.version != owner.generator_version
                        || generator.parameters != owner.base_parameters
                })
            {
                model_uploading.set(false);
                feedback.set(Some(ScopedFeedback {
                    owner,
                    message: format!("{error_prefix}: The selected generator settings changed while the model was being saved."),
                }));
                return;
            }
            let asset_id = loop {
                let candidate = format!("generator-model-{}", runtime.operation().0);
                if !current
                    .document
                    .assets
                    .iter()
                    .any(|asset| asset.id == candidate)
                {
                    break candidate;
                }
            };
            let asset = Asset {
                id: asset_id.clone(),
                name: imported.filename.clone(),
                media_type: imported.media_type.clone(),
                sha256: imported.sha256.clone(),
                license: None,
                source: Some("local file".into()),
            };
            let mut candidate = latest_definition;
            candidate
                .generator
                .as_mut()
                .expect("validated generator")
                .parameters
                .insert(
                    parameter,
                    Value::String(format!("boardstudio-asset:{asset_id}")),
                );
            let view = GeneratorView {
                scope: runtime.scope(),
                selection: selected(),
                scope_generation: scope_generation(),
                selection_generation: selection_generation(),
                workspace: workspace(),
            };
            let operation_id = runtime.operation();
            let event = match prepare_generator_asset_edit(
                &current,
                &owner,
                &view,
                candidate.clone(),
                asset,
                operation_id,
            ) {
                Ok(event) => event,
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
            model_uploading.set(false);
            let outcome = runtime.observe_operation(operation_id);
            pending_apply.set(Some(PendingApply {
                owner,
                candidate,
                outcome,
            }));
            feedback.set(None);
            runtime.submit(event);
        });
    });
    let on_apply = {
        let runtime = runtime.clone();
        let mut pending_apply = pending_apply;
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
        "3D model placement",
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
                            disabled: pending_apply().is_some() || model_uploading(),
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
                    GeneratorPreviewStatus::Pending => rsx! { p { role: "status", "Generating footprint preview…" } },
                    GeneratorPreviewStatus::Ready => rsx! { p { role: "status", "Current generator preview is ready to apply." } },
                    GeneratorPreviewStatus::Failed(error) => rsx! { p { class: "m1-parts-load-error", role: "alert", "Generator preview failed: {error}" } },
                }
            }
            if model_uploading() { p { role: "status", "Reading and saving generator model…" } }
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
    on_import: EventHandler<(String, web_sys::File)>,
    busy: bool,
    feedback: Option<String>,
) -> Element {
    let key = entry.key.clone();
    let label = entry.label.clone();
    if key.to_ascii_lowercase().ends_with("3dmodel_filename") {
        let current = input_value(&value, &entry.kind);
        let field_error = feedback.filter(|message| {
            message
                .to_ascii_lowercase()
                .starts_with(&key.replace('_', " ").to_ascii_lowercase())
        });
        return rsx! {
            div { class: "m1-generator-field m1-generator-model-field",
                label { "{label}",
                    input {
                        r#type: "text",
                        aria_label: "{key}",
                        value: "{current}",
                        readonly: true,
                    }
                }
                label { class: "m1-generator-field", "Attach STEP / STL / WRL",
                    input {
                        r#type: "file",
                        accept: ".step,.stp,.stl,.wrl,model/step,model/stl,model/vrml",
                        disabled: busy,
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
                }
                if !current.is_empty() {
                    button {
                        r#type: "button",
                        class: "m1-generator-apply",
                        disabled: busy,
                        onclick: move |_| on_change.call(Value::String(String::new())),
                        "Remove model"
                    }
                }
                if let Some(message) = field_error { p { class: "m1-parts-load-error", role: "alert", "{message}" } }
            }
        };
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{MechanicalPartProfile, ProjectDoc, SceneDelta};
    use futures_channel::oneshot;
    use std::sync::Arc;
    use std::{cell::RefCell, collections::VecDeque};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    const DEFINITION_ID: &str = "generator:ceoloide/switch_mx";

    struct CandidateRequest {
        definition: PartDefinition,
        schema: BTreeMap<String, Value>,
        edits: BTreeMap<String, Value>,
        complete: oneshot::Sender<Result<PartDefinition, String>>,
        returned: oneshot::Receiver<()>,
    }

    #[derive(Clone)]
    struct GeneratorMountedFixture {
        snapshot: AcceptedSnapshot,
        scope: Scope,
        first: PartDefinition,
        second: PartDefinition,
        runtime: Rc<Runtime>,
        requests: Rc<RefCell<VecDeque<CandidateRequest>>>,
    }

    #[derive(Clone, Copy)]
    struct GeneratorMountedSignals {
        definition: Signal<PartDefinition>,
        selected: Signal<Option<(Option<Scope>, String)>>,
        selection_generation: Signal<u64>,
        store: Signal<Option<GeneratorPreviewDraft>>,
    }

    #[component]
    fn GeneratorMountedHost() -> Element {
        let fixture = use_context::<Rc<GeneratorMountedFixture>>();
        let handles = use_context::<Rc<RefCell<Option<GeneratorMountedSignals>>>>();
        let definition = use_signal(|| fixture.first.clone());
        let selected = use_signal(|| Some((Some(fixture.scope.clone()), fixture.first.id.clone())));
        let selection_generation = use_signal(|| 1_u64);
        let scope_generation = use_signal(|| 1_u64);
        let workspace = use_signal(|| "Parts");
        let store = use_signal(|| None::<GeneratorPreviewDraft>);
        let selected_context = use_signal(|| None);
        let anchor_scope = use_signal(|| Some(fixture.scope.clone()));
        let adapter = use_hook(|| {
            super::super::super::selection::SelectionAdapter::new(
                selected_context,
                anchor_scope,
                scope_generation,
            )
        });
        use_context_provider(|| adapter.clone());
        use_context_provider(|| super::super::PartsSelectionGeneration(selection_generation));
        use_context_provider(|| super::super::GeneratorDraftStore(store));
        use_context_provider(|| super::super::super::WorkspaceState(workspace));
        use_context_provider(|| Signal::new(0_u64));
        *handles.borrow_mut() = Some(GeneratorMountedSignals {
            definition,
            selected,
            selection_generation,
            store,
        });
        rsx! {
            GeneratorSettingsEditor {
                snapshot: fixture.snapshot.clone(),
                scope: Some(fixture.scope.clone()),
                selected,
                definition: definition(),
            }
        }
    }

    fn controlled_candidate_provider(
        requests: Rc<RefCell<VecDeque<CandidateRequest>>>,
    ) -> GeneratorCandidateTestProvider {
        GeneratorCandidateTestProvider(Rc::new(move |definition, schema, edits| {
            let (complete, result) = oneshot::channel();
            let (returned, returned_signal) = oneshot::channel();
            requests.borrow_mut().push_back(CandidateRequest {
                definition,
                schema,
                edits,
                complete,
                returned: returned_signal,
            });
            Box::pin(async move {
                let result = result
                    .await
                    .unwrap_or_else(|_| Err("controlled preview was retired".into()));
                let _ = returned.send(());
                result
            })
        }))
    }

    async fn mount_generator_editor() -> (
        web_sys::Element,
        Rc<GeneratorMountedFixture>,
        Rc<RefCell<Option<GeneratorMountedSignals>>>,
    ) {
        let first = definition();
        let mut second = first.clone();
        second.id = "test:other-mx-switch".into();
        second.name = "Other MX switch".into();
        let snapshot = snapshot(vec![first.clone(), second.clone()], 4);
        let scope = scope();
        let runtime = Runtime::new().expect("browser Runtime fixture initializes");
        runtime.set_definition_name_test_state(snapshot.clone(), Some(scope.clone()));
        let requests = Rc::new(RefCell::new(VecDeque::new()));
        let fixture = Rc::new(GeneratorMountedFixture {
            snapshot,
            scope,
            first,
            second,
            runtime: runtime.clone(),
            requests: requests.clone(),
        });
        let handles = Rc::new(RefCell::new(None));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("parts-generator-owner-mounted-test");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(GeneratorMountedHost);
        dom.provide_root_context(fixture.clone());
        dom.provide_root_context(handles.clone());
        dom.provide_root_context(runtime);
        dom.provide_root_context(controlled_candidate_provider(requests));
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_for_selector(&root, "input[aria-label='Keycap Width']").await;
        (root, fixture, handles)
    }

    async fn wait_for_selector(root: &web_sys::Element, selector: &str) {
        for _ in 0..150 {
            if root.query_selector(selector).unwrap().is_some() {
                return;
            }
            gloo_timers::future::TimeoutFuture::new(20).await;
        }
        panic!(
            "timed out waiting for mounted selector {selector}; mounted text was: {}",
            root.text_content().unwrap_or_default()
        );
    }

    async fn take_request(
        requests: &Rc<RefCell<VecDeque<CandidateRequest>>>,
        width: f64,
    ) -> CandidateRequest {
        for _ in 0..150 {
            let index =
                {
                    let queue = requests.borrow();
                    queue.iter().position(|request| {
                        request.edits.get("keycap_width").and_then(|value| {
                            value.as_f64().or_else(|| value.as_str()?.parse().ok())
                        }) == Some(width)
                    })
                };
            if let Some(index) = index {
                return requests.borrow_mut().remove(index).unwrap();
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("no controlled candidate request arrived for width {width}");
    }

    fn dispatch_width(root: &web_sys::Element, width: &str) {
        let input = root
            .query_selector("input[aria-label='Keycap Width']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        input.set_value(width);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
    }

    fn candidate(request: &CandidateRequest) -> PartDefinition {
        apply_input(&request.definition, &request.schema, &request.edits)
            .expect("test width is valid against the retained parameter schema")
    }

    async fn finish_request(request: CandidateRequest, result: Result<PartDefinition, String>) {
        request.complete.send(result).unwrap();
        request.returned.await.expect("candidate provider returned");
    }

    async fn wait_for_text(root: &web_sys::Element, selector: &str, text: &str) {
        for _ in 0..150 {
            let found = root
                .query_selector(selector)
                .unwrap()
                .and_then(|element| element.text_content())
                .is_some_and(|content| content.contains(text));
            if found {
                return;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("timed out waiting for `{text}` in {selector}");
    }

    async fn take_initial_default_request(
        requests: &Rc<RefCell<VecDeque<CandidateRequest>>>,
    ) -> CandidateRequest {
        for _ in 0..150 {
            if let Some(index) = {
                let queue = requests.borrow();
                queue.iter().position(|request| {
                    request.edits.get("side") == Some(&Value::String("F".into()))
                })
            } {
                return requests.borrow_mut().remove(index).unwrap();
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("initial mounted generator preview did not request the schema default side F");
    }

    fn definition() -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id": DEFINITION_ID,
            "name": "MX switch",
            "kind": "switch",
            "courtyard": [
                {"x": -7.0, "y": -4.0},
                {"x": 7.0, "y": -4.0},
                {"x": 7.0, "y": 4.0}
            ],
            "pads": [{
                "id": "pad-1", "number": "1", "at": {"x": 0.0, "y": 0.0},
                "size": {"x": 2.0, "y": 2.0}, "shape": "circle"
            }],
            "generator": {
                "source": "ceoloide/switch_mx", "version": "bundled-1",
                "parameters": {"keycap_width": 18, "keycap_height": 18}
            }
        }))
        .expect("generator definition")
    }

    fn snapshot(definitions: Vec<PartDefinition>, revision: u64) -> AcceptedSnapshot {
        let mut document = ProjectDoc::empty("generator-merge", "Sofle v2");
        document.revision = revision;
        document.definitions = definitions;
        let scene: SceneDelta = serde_json::from_value(serde_json::json!({
            "revision": revision,
            "transactionId": "generator-merge-fixture",
            "changedIds": [], "transforms": [], "matrixScenes": [],
            "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
            "readiness": {"layout": true, "outline": true, "pcb": true, "case": false}
        }))
        .expect("scene");
        AcceptedSnapshot {
            token: SnapshotToken(revision + 10),
            session_epoch: SessionEpoch(3),
            document: Arc::new(document),
            scene: Arc::new(scene),
        }
    }

    fn scope() -> Scope {
        Scope {
            session_epoch: SessionEpoch(3),
            document_id: "generator-merge".into(),
            board_id: "left-pcb".into(),
            instance_id: None,
        }
    }

    fn view(scope: &Scope) -> GeneratorView {
        GeneratorView {
            scope: Some(scope.clone()),
            selection: Some((Some(scope.clone()), DEFINITION_ID.into())),
            scope_generation: 11,
            selection_generation: 7,
            workspace: "Parts",
        }
    }

    fn mechanical_profile() -> MechanicalPartProfile {
        MechanicalPartProfile {
            source_geometry: None,
            pcb_holes: None,
            clearance_volumes: None,
            openings: None,
            clearances: None,
            supported_thickness: None,
            switch_family: None,
            definition_id: DEFINITION_ID.into(),
            source: "latest accepted metadata".into(),
            cutouts: Vec::new(),
            plate_to_pcb: 0.8,
        }
    }

    fn changed_candidate(base: &PartDefinition) -> PartDefinition {
        let mut candidate = base.clone();
        let generator = candidate.generator.as_mut().expect("generator");
        generator
            .parameters
            .insert("keycap_width".into(), Value::from(20));
        candidate.pads[0].size.x = 3.0;
        candidate.courtyard[0].x = -8.0;
        candidate
    }

    #[wasm_bindgen_test]
    fn retained_generator_apply_merges_latest_metadata_and_rejects_deleted_owner() {
        let base = definition();
        let base_snapshot = snapshot(vec![base.clone()], 4);
        let scope = scope();
        let selection = Some((Some(scope.clone()), DEFINITION_ID.into()));
        let owner = make_owner(&base_snapshot, Some(scope.clone()), selection, &base, 11, 7)
            .expect("owner");
        assert!(owner.project_owned_at_start);

        let mut latest = base.clone();
        latest.name = "Latest accepted name".into();
        latest.mechanical_profile = Some(mechanical_profile());
        let latest_snapshot = snapshot(vec![latest.clone()], 5);
        let current_owner = make_owner(
            &latest_snapshot,
            Some(scope.clone()),
            Some((Some(scope.clone()), DEFINITION_ID.into())),
            &latest,
            11,
            7,
        )
        .expect("current owner");
        assert_eq!(
            owner, current_owner,
            "non-generator metadata keeps the draft owner"
        );

        let bundled_snapshot = snapshot(Vec::new(), 4);
        let bundled_owner = make_owner(
            &bundled_snapshot,
            Some(scope.clone()),
            Some((Some(scope.clone()), DEFINITION_ID.into())),
            &base,
            11,
            7,
        )
        .expect("bundled owner");
        assert!(!bundled_owner.project_owned_at_start);
        assert_eq!(
            bundled_owner, current_owner,
            "materializing a bundled definition keeps the logical draft owner"
        );

        let candidate = changed_candidate(&base);
        let event = prepare_generator_edit(
            &latest_snapshot,
            &owner,
            &view(&scope),
            candidate.clone(),
            OperationId(701),
        )
        .expect("candidate rebases onto the latest accepted document");
        let Event::Edit { command, .. } = event else {
            panic!("Apply should submit one ordinary edit");
        };
        assert_eq!(command.base_revision, 5);
        let EditOperation::ReplaceDocument { document } = command.operation else {
            panic!("Apply should replace the current document");
        };
        let applied = document
            .definitions
            .iter()
            .find(|definition| definition.id == DEFINITION_ID)
            .expect("applied definition");
        assert_eq!(applied.name, "Latest accepted name");
        assert_eq!(applied.mechanical_profile, latest.mechanical_profile);
        assert_eq!(applied.generator, candidate.generator);
        assert_eq!(applied.pads, candidate.pads);
        assert_eq!(applied.courtyard, candidate.courtyard);

        let deleted_snapshot = snapshot(Vec::new(), 6);
        assert!(
            prepare_generator_edit(
                &deleted_snapshot,
                &owner,
                &view(&scope),
                candidate,
                OperationId(702),
            )
            .is_err()
        );
    }

    #[wasm_bindgen_test]
    async fn mounted_generator_provider_failure_is_visible_and_never_enables_apply() {
        let (root, fixture, _) = mount_generator_editor().await;
        dispatch_width(&root, "19");
        wait_for_text(&root, "[role='status']", "Generating footprint preview").await;
        let request = take_request(&fixture.requests, 19.0).await;
        assert!(
            root.query_selector(".m1-generator-settings > button.m1-generator-apply")
                .unwrap()
                .unwrap()
                .has_attribute("disabled")
        );

        finish_request(request, Err("controlled candidate provider failure".into())).await;
        wait_for_text(
            &root,
            "[role='alert']",
            "controlled candidate provider failure",
        )
        .await;
        assert!(
            root.query_selector(".m1-generator-settings > button.m1-generator-apply")
                .unwrap()
                .unwrap()
                .has_attribute("disabled")
        );
        let accepted = fixture.runtime.model().accepted.unwrap();
        assert_eq!(accepted.document, fixture.snapshot.document);
        assert_eq!(
            accepted.document.revision,
            fixture.snapshot.document.revision
        );
        assert_eq!(
            accepted.document.definitions[0]
                .generator
                .as_ref()
                .unwrap()
                .parameters["keycap_width"],
            18
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_generator_initial_schema_default_seeds_unapplied_preview() {
        let (root, fixture, handles) = mount_generator_editor().await;
        let side = root.query_selector("select[aria-label='side']").unwrap();
        let side = side.expect("mounted Board side control");
        assert_eq!(
            js_sys::Reflect::get(side.as_ref(), &"value".into())
                .unwrap()
                .as_string()
                .as_deref(),
            Some("F")
        );

        let request = take_initial_default_request(&fixture.requests).await;
        assert_eq!(request.edits.get("side"), Some(&Value::String("F".into())));
        let candidate =
            super::super::catalogue::normalize_generator_definition(candidate(&request))
                .await
                .expect("packaged normalizer prepares the default-side draft");
        assert_eq!(
            candidate.pads.len(),
            7,
            "real MX hotswap geometry replaces the catalogue fixture"
        );
        let pad_one = candidate.pads.iter().find(|pad| pad.number == "1").unwrap();
        let pad_two = candidate.pads.iter().find(|pad| pad.number == "2").unwrap();
        assert!(pad_one.at.x > 0.0, "Front preview places pad 1 on the right");
        assert!(pad_two.at.x < 0.0, "Front preview places pad 2 on the left");
        finish_request(request, Ok(candidate.clone())).await;
        wait_for_text(
            &root,
            "[role='status']",
            "Current generator preview is ready",
        )
        .await;
        let draft = (handles.borrow().as_ref().unwrap().store)().unwrap();
        assert_eq!(draft.status, GeneratorPreviewStatus::Ready);
        assert_eq!(draft.definition.as_ref(), Some(&candidate));
        assert_eq!(
            draft
                .definition
                .as_ref()
                .unwrap()
                .generator
                .as_ref()
                .unwrap()
                .parameters["side"],
            "F"
        );
        let accepted = fixture.runtime.model().accepted.unwrap();
        assert_eq!(accepted.document, fixture.snapshot.document);
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_generator_discards_superseded_edit_and_selection_results() {
        let (root, fixture, handles) = mount_generator_editor().await;
        let initial_request = take_initial_default_request(&fixture.requests).await;
        dispatch_width(&root, "19");
        let older_edit = take_request(&fixture.requests, 19.0).await;
        dispatch_width(&root, "20");
        let newer_edit = take_request(&fixture.requests, 20.0).await;
        let newer_candidate = candidate(&newer_edit);
        finish_request(newer_edit, Ok(newer_candidate)).await;
        wait_for_text(
            &root,
            "[role='status']",
            "Current generator preview is ready",
        )
        .await;
        let ready = (handles.borrow().as_ref().unwrap().store)().unwrap();
        assert_eq!(
            ready
                .definition
                .as_ref()
                .unwrap()
                .generator
                .as_ref()
                .unwrap()
                .parameters["keycap_width"]
                .as_f64(),
            Some(20.0)
        );

        let older_candidate = candidate(&older_edit);
        finish_request(older_edit, Ok(older_candidate)).await;
        let initial_candidate = candidate(&initial_request);
        finish_request(initial_request, Ok(initial_candidate)).await;
        let still_ready = (handles.borrow().as_ref().unwrap().store)().unwrap();
        assert_eq!(
            still_ready
                .definition
                .as_ref()
                .unwrap()
                .generator
                .as_ref()
                .unwrap()
                .parameters["keycap_width"]
                .as_f64(),
            Some(20.0)
        );

        dispatch_width(&root, "21");
        let retired_selection = take_request(&fixture.requests, 21.0).await;
        let mut mounted = handles.borrow().as_ref().copied().unwrap();
        mounted.definition.set(fixture.second.clone());
        mounted.selected.set(Some((
            Some(fixture.scope.clone()),
            fixture.second.id.clone(),
        )));
        mounted
            .selection_generation
            .with_mut(|generation| *generation += 1);
        let new_selection = take_initial_default_request(&fixture.requests).await;
        assert_eq!(new_selection.definition.id, fixture.second.id);
        let pending = (handles.borrow().as_ref().unwrap().store)().unwrap();
        assert_eq!(pending.owner.definition_id, fixture.second.id);
        assert_eq!(pending.status, GeneratorPreviewStatus::Pending);
        let retired_candidate = candidate(&retired_selection);
        finish_request(retired_selection, Ok(retired_candidate)).await;
        let still_pending = (handles.borrow().as_ref().unwrap().store)().unwrap();
        assert_eq!(
            still_pending, pending,
            "retired selection cannot replace the new pending draft"
        );
        let new_candidate = candidate(&new_selection);
        finish_request(new_selection, Ok(new_candidate.clone())).await;
        wait_for_text(
            &root,
            "[role='status']",
            "Current generator preview is ready",
        )
        .await;
        let current = (handles.borrow().as_ref().unwrap().store)().unwrap();
        assert_eq!(current.owner.definition_id, fixture.second.id);
        assert_eq!(current.status, GeneratorPreviewStatus::Ready);
        assert_eq!(current.definition.as_ref(), Some(&new_candidate));
        assert_eq!(
            fixture.runtime.model().accepted.unwrap().document,
            fixture.snapshot.document
        );
        root.remove();
    }
}
