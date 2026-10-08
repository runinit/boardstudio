//! Parts-owned retained generator settings drafts and their accepted edit boundary.
use super::{GeneratorDraftStore, GeneratorPreviewDraft};
use crate::parts_custom_definition::replacement_commit;
use crate::{presentation::model_asset_import::read_model_file, runtime::Runtime};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope};
use boardstudio_core::model::{Asset, EditOperation, Net, PartDefinition, Pin};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
#[cfg(test)]
use boardstudio_web_runtime::pending_edits::PendingEdits;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
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

/// The apply or upload this panel observes while its edit is pending: the helper owns
/// the observation, the panel keeps the follow-up context (whose owner, which draft
/// generation, which uploaded parameter).
#[derive(Clone)]
struct GeneratorAction {
    owner: GeneratorOwner,
    draft_sequence: u64,
    apply: bool,
    uploaded_parameter: Option<String>,
}

/// This panel's two bounded one-shot keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GeneratorKey {
    Apply,
    Upload,
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

fn remap_terminal_nets(
    document: &boardstudio_core::model::ProjectDoc,
    original: &PartDefinition,
    candidate: &PartDefinition,
) -> Result<Vec<Net>, String> {
    let instances = document
        .parts
        .iter()
        .filter(|part| part.definition_id == candidate.id)
        .collect::<Vec<_>>();
    let mut assignments = Vec::<(String, Vec<String>, Vec<String>, String)>::new();
    for part in instances {
        for (terminal, old_pads) in &original.terminals {
            let net_ids = document
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
    for net in &document.nets {
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
    let mut nets = document.nets.clone();
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

/// Resolve one generator Apply against the accepted document at execution: the accepted
/// definition is found (or materialized from the draft's base for a bundled definition),
/// the candidate is rebased onto it, terminal nets are remapped, and the replacement is
/// cloned from the accepted document, so an Apply queued behind another edit never
/// reverts it. Vanished or ineligible targets retire with a reason.
pub fn generator_apply_resolver(
    owner: GeneratorOwner,
    candidate: PartDefinition,
    changed_parameters: BTreeSet<String>,
) -> EditResolver {
    EditResolver::new(
        "parts-generator-settings",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            if accepted.document.id != owner.document_id
                || candidate.id != owner.definition_id
                || candidate.generator.as_ref().is_none_or(|generator| {
                    generator.source != owner.source || generator.version != owner.generator_version
                })
            {
                return Resolution::Retire(
                    "The selected generator definition changed while these settings were open."
                        .into(),
                );
            }
            match generator_replacement(document, &owner, &candidate, &changed_parameters) {
                Ok(replacement) => replacement,
                Err(reason) => Resolution::Retire(reason),
            }
        },
    )
}

/// Resolve one generator model upload against the accepted document at execution. The
/// bytes were stored by hash before the edit was submitted; this resolver only adds the
/// `Asset` metadata, choosing an identity that is unique against the accepted assets at
/// execution, and binds it into the generator parameters.
pub fn generator_model_upload_resolver(
    owner: GeneratorOwner,
    candidate: PartDefinition,
    parameter: String,
    asset: Asset,
    asset_seed: u64,
) -> EditResolver {
    EditResolver::new(
        "parts-generator-settings",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            if accepted.document.id != owner.document_id
                || candidate.id != owner.definition_id
                || candidate.generator.is_none()
            {
                return Resolution::Retire(
                    "The selected generator definition changed while the model was being saved."
                        .into(),
                );
            }
            let asset_id = unique_generator_model_id(document, asset_seed);
            let mut bound = document
                .definitions
                .iter()
                .find(|definition| definition.id == owner.definition_id)
                .cloned()
                .unwrap_or_else(|| candidate.clone());
            let Some(generator) = bound.generator.as_mut() else {
                return Resolution::Retire("The part is no longer generator-backed.".into());
            };
            generator.parameters.insert(
                parameter.clone(),
                Value::String(format!("boardstudio-asset:{asset_id}")),
            );
            let mut target_ids = match generator_replacement(
                document,
                &owner,
                &bound,
                &BTreeSet::from([parameter.clone()]),
            ) {
                Ok(replacement) => replacement,
                Err(reason) => return Resolution::Retire(reason),
            };
            let Resolution::Submit(command) = &mut target_ids else {
                return target_ids;
            };
            let EditOperation::ReplaceDocument { document } = &mut command.operation else {
                return Resolution::Retire(
                    "The model attachment did not produce a document replacement.".into(),
                );
            };
            if document
                .assets
                .iter()
                .any(|existing| existing.id == asset_id)
            {
                return Resolution::Retire("The model asset identity is already in use.".into());
            }
            let mut asset = asset.clone();
            asset.id = asset_id.clone();
            document.assets.push(asset);
            command.target_ids.push(asset_id);
            target_ids
        },
    )
}

/// The replacement document for one generator candidate, or the retire reason. The
/// candidate is rebased onto the accepted definition so unrelated accepted metadata
/// survives, and equality with the accepted document resolves `Unchanged`.
fn generator_replacement(
    document: &boardstudio_core::model::ProjectDoc,
    owner: &GeneratorOwner,
    candidate: &PartDefinition,
    changed_parameters: &BTreeSet<String>,
) -> Result<Resolution, String> {
    let accepted_definition = document
        .definitions
        .iter()
        .find(|definition| definition.id == owner.definition_id);
    if accepted_definition.is_none() && owner.project_owned_at_start {
        return Err("The selected project generator definition was removed.".into());
    }
    let original = accepted_definition
        .cloned()
        .unwrap_or_else(|| owner.base_definition.clone());
    if original.generator.as_ref().is_none_or(|generator| {
        generator.source != owner.source || generator.version != owner.generator_version
    }) {
        return Err(
            "The selected generator definition changed while these settings were open.".into(),
        );
    }
    let rebased = rebase_generator_candidate(
        &owner.base_definition,
        &original,
        candidate,
        changed_parameters,
    )?;
    let rebased = boardstudio_core::generators::normalize_definition(rebased)?;
    let nets = remap_terminal_nets(document, &original, &rebased)?;
    let mut replacement = document.clone();
    if let Some(existing) = replacement
        .definitions
        .iter_mut()
        .find(|definition| definition.id == rebased.id)
    {
        *existing = rebased.clone();
    } else {
        replacement.definitions.push(rebased.clone());
    }
    replacement.nets = nets;
    let mut target_ids = vec![rebased.id.clone()];
    target_ids.extend(
        replacement
            .parts
            .iter()
            .filter(|part| part.definition_id == rebased.id)
            .map(|part| part.id.clone()),
    );
    // The replacement is built from the accepted document, so equality means this
    // candidate already holds: resolve Unchanged, not a landing heuristic (ADR-0005).
    if replacement == *document {
        return Ok(Resolution::Unchanged);
    }
    Ok(replacement_commit(
        EditOperation::ReplaceDocument {
            document: Box::new(replacement),
        },
        target_ids,
    ))
}

fn unique_generator_model_id(document: &boardstudio_core::model::ProjectDoc, seed: u64) -> String {
    let root = format!("generator-model-{seed}");
    if !document.assets.iter().any(|asset| asset.id == root) {
        return root;
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{root}-{suffix}");
        if !document.assets.iter().any(|asset| asset.id == candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn merge_generator_field<T: Clone + PartialEq>(base: &T, candidate: &T, rebased: &mut T) {
    if candidate != base {
        *rebased = candidate.clone();
    }
}

fn rebase_generator_candidate(
    base: &PartDefinition,
    latest: &PartDefinition,
    candidate: &PartDefinition,
    changed_parameters: &BTreeSet<String>,
) -> Result<PartDefinition, String> {
    if base.id != latest.id || candidate.id != latest.id {
        return Err("The selected generator definition changed identity.".into());
    }
    let mut rebased = latest.clone();
    merge_generator_field(
        &base.hardware_profile,
        &candidate.hardware_profile,
        &mut rebased.hardware_profile,
    );
    merge_generator_field(
        &base.input_profile,
        &candidate.input_profile,
        &mut rebased.input_profile,
    );
    merge_generator_field(&base.name, &candidate.name, &mut rebased.name);
    merge_generator_field(&base.kind, &candidate.kind, &mut rebased.kind);
    merge_generator_field(&base.keycap, &candidate.keycap, &mut rebased.keycap);
    merge_generator_field(
        &base.envelope_source,
        &candidate.envelope_source,
        &mut rebased.envelope_source,
    );
    merge_generator_field(
        &base.kicad_source,
        &candidate.kicad_source,
        &mut rebased.kicad_source,
    );
    merge_generator_field(
        &base.terminals,
        &candidate.terminals,
        &mut rebased.terminals,
    );
    merge_generator_field(
        &base.matrix_terminals,
        &candidate.matrix_terminals,
        &mut rebased.matrix_terminals,
    );
    merge_generator_field(
        &base.envelope_notice,
        &candidate.envelope_notice,
        &mut rebased.envelope_notice,
    );
    merge_generator_field(
        &base.courtyard,
        &candidate.courtyard,
        &mut rebased.courtyard,
    );
    merge_generator_field(&base.pads, &candidate.pads, &mut rebased.pads);
    merge_generator_field(&base.models, &candidate.models, &mut rebased.models);
    if let (Some(_base), Some(latest), Some(candidate), Some(target)) = (
        &base.generator,
        &latest.generator,
        &candidate.generator,
        rebased.generator.as_mut(),
    ) {
        // Apply commits changed settings; unchanged keys retain preceding uploads.
        target.source = candidate.source.clone();
        target.version = candidate.version.clone();
        target.parameters = latest.parameters.clone();
        for (key, value) in &candidate.parameters {
            if changed_parameters.contains(key) || !latest.parameters.contains_key(key) {
                target.parameters.insert(key.clone(), value.clone());
            }
        }
    }
    merge_generator_field(
        &base.mechanical_profile,
        &candidate.mechanical_profile,
        &mut rebased.mechanical_profile,
    );
    Ok(rebased)
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
    let mut changed_parameters = use_signal(BTreeSet::<String>::new);
    let mut feedback = use_signal(|| None::<ScopedFeedback>);
    let pending_edits = use_hook(|| PendingEditSignals::<GeneratorKey>::new());
    let apply_intent = use_signal(|| None::<GeneratorAction>);
    let upload_intent = use_signal(|| None::<GeneratorAction>);
    let mut uploaded_preview_generation = use_signal(|| 0_u64);
    let sequence = use_hook(|| Rc::new(Cell::new(0_u64)));
    let draft_generation = use_hook(|| Rc::new(Cell::new(0_u64)));
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

    use_effect(use_reactive((&version(),), {
        let runtime = runtime.clone();
        let sequence = sequence.clone();
        let draft_generation = draft_generation.clone();
        let pending_edits = pending_edits.clone();
        let mut feedback = feedback;
        let mut store = store;
        let mut edits = edits;
        let mut changed_parameters = changed_parameters;
        let mut apply_intent = apply_intent;
        let mut upload_intent = upload_intent;
        move |_| {
            let intents = [
                (GeneratorKey::Apply, apply_intent.peek().clone()),
                (GeneratorKey::Upload, upload_intent.peek().clone()),
            ];
            let Some(observed_owner) = intents
                .iter()
                .find_map(|(_, intent)| intent.as_ref().map(|action| action.owner.clone()))
            else {
                return;
            };
            // The editor outlives selection changes, so owner liveness is the context
            // owner (workspace, scope, selection, generations), not the accepted
            // generator parameters the edit itself is about to change. Both actions
            // belong to this editor instance, so one answer drains the collection.
            let owner_is_live = owner_context_is_current(
                &observed_owner,
                &runtime,
                selected,
                scope_generation(),
                selection_generation(),
                workspace(),
            );
            for result in pending_edits.settle(owner_is_live, |_| String::new()) {
                let intent = match &result {
                    PendingEditResult::Landed { key, .. }
                    | PendingEditResult::Failed { key, .. }
                    | PendingEditResult::Retired { key } => match key {
                        GeneratorKey::Apply => apply_intent.take(),
                        GeneratorKey::Upload => upload_intent.take(),
                    },
                };
                let Some(intent) = intent else {
                    continue;
                };
                match result {
                    // Landed means landed: drop the draft store without re-checking the
                    // accepted document for the requested values.
                    PendingEditResult::Landed { .. } => {
                        if intent.apply && draft_generation.get() == intent.draft_sequence {
                            sequence.set(sequence.get().wrapping_add(1));
                            store.set(None);
                            edits.set(BTreeMap::new());
                            changed_parameters.set(BTreeSet::new());
                        }
                        if let Some(parameter) = &intent.uploaded_parameter {
                            // Upload owns this disabled file control until settlement;
                            // reveal its accepted value without erasing other draft fields.
                            edits.write().remove(parameter);
                            changed_parameters.write().remove(parameter);
                            sequence.set(sequence.get().wrapping_add(1));
                            store.set(None);
                            uploaded_preview_generation += 1;
                        }
                        feedback.set(None);
                    }
                    PendingEditResult::Failed { message, .. } => {
                        if intent.apply && draft_generation.get() == intent.draft_sequence {
                            sequence.set(sequence.get().wrapping_add(1));
                            store.set(None);
                            edits.set(BTreeMap::new());
                            changed_parameters.set(BTreeSet::new());
                        }
                        feedback.set(Some(ScopedFeedback {
                            owner: intent.owner.clone(),
                            message,
                        }));
                    }
                    PendingEditResult::Retired { .. } => {}
                }
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
    let user_draft_generation = draft_generation.clone();
    let change_parameter = use_callback(move |(key, value): (String, Value)| {
        user_draft_generation.set(user_draft_generation.get().wrapping_add(1));
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
    let import_pending_edits = pending_edits.clone();
    let import_generator_model = use_callback(move |(parameter, file): (String, web_sys::File)| {
        let pending_edits = import_pending_edits.clone();
        let runtime = model_import_runtime.clone();
        let owner = model_import_owner.clone();
        if model_uploading()
            || pending_edits.is_pending(&GeneratorKey::Upload)
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
        let pending_edits = pending_edits.clone();
        let mut upload_intent = upload_intent;
        let draft_sequence = 0;
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
            pending_edits.begin_one_shot(
                &runtime,
                GeneratorKey::Upload,
                "parts-generator-settings",
                Some("generator".into()),
                generator_model_upload_resolver(
                    owner.clone(),
                    latest_definition,
                    parameter.clone(),
                    asset,
                    asset_seed,
                ),
            );
            upload_intent.set(Some(GeneratorAction {
                owner,
                draft_sequence,
                apply: false,
                uploaded_parameter: Some(parameter),
            }));
            feedback.set(None);
        });
    });
    let on_apply = {
        let runtime = runtime.clone();
        let pending_edits = pending_edits.clone();
        let mut apply_intent = apply_intent;
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
            if pending_edits.is_pending(&GeneratorKey::Apply)
                || !owner_context_is_current(
                    &current_owner,
                    &runtime,
                    selected,
                    scope_generation(),
                    selection_generation(),
                    workspace(),
                )
            {
                return;
            }
            pending_edits.begin_one_shot(
                &runtime,
                GeneratorKey::Apply,
                "parts-generator-settings",
                Some("generator settings".into()),
                generator_apply_resolver(current_owner.clone(), candidate, changed_parameters()),
            );
            apply_intent.set(Some(GeneratorAction {
                owner: current_owner.clone(),
                draft_sequence: draft_generation.get(),
                apply: true,
                uploaded_parameter: None,
            }));
            feedback.set(None);
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
                            for entry in group_entries {
                                GeneratorParameterField {
                                    entry: entry.clone(),
                                    value: edits().get(&entry.key).cloned().unwrap_or(entry.value.clone()),
                                    on_change: EventHandler::new({ let key = entry.key.clone(); move |value| change_parameter.call((key.clone(), value)) }),
                                    on_import: import_generator_model,
                                    busy: model_uploading() || pending_edits.is_pending(&GeneratorKey::Upload),
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
                disabled: !preview_ready || pending_edits.is_pending(&GeneratorKey::Apply),
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
    use crate::runtime::project_name_test_support as support;
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{Board, MechanicalPartProfile, ProjectDoc, SceneDelta};
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
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        let _ = version();
        use_hook({
            let runtime = fixture.runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let current = fixture.runtime.model().accepted.unwrap();
        let accepted_definition = current
            .document
            .definitions
            .iter()
            .find(|item| item.id == definition().id)
            .cloned()
            .unwrap_or_else(|| definition());
        *handles.borrow_mut() = Some(GeneratorMountedSignals {
            definition,
            selected,
            selection_generation,
            store,
        });
        rsx! {
            GeneratorSettingsEditor {
                snapshot: current,
                scope: Some(fixture.scope.clone()),
                selected,
                definition: accepted_definition,
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
        let mut document = ProjectDoc::empty("generator-merge", "Sofle v2");
        document.revision = 4;
        document.definitions = vec![first.clone(), second.clone()];
        document.boards.push(Board {
            id: "left-pcb".into(),
            name: "Left PCB".into(),
            outline_ids: Vec::new(),
            part_ids: Vec::new(),
            net_ids: Vec::new(),
            thickness: 1.6,
            traces: Vec::new(),
            vias: Vec::new(),
        });
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let snapshot = runtime
            .model()
            .accepted
            .expect("the generator project is accepted");
        let scope = runtime
            .scope()
            .expect("the accepted generator project has a scope");
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
                "parameters": {"keycap_width": 18.0, "keycap_height": 18}
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
        boardstudio_core::generators::normalize_definition(candidate).unwrap()
    }

    #[wasm_bindgen_test]
    async fn retained_generator_apply_merges_latest_metadata_and_rejects_deleted_owner() {
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
        let runtime = support::new_runtime();
        support::open_document(&runtime, latest_snapshot.document.as_ref().clone()).await;
        let accepted = runtime.model().accepted.unwrap();
        let mut owner = owner;
        owner.session_epoch = accepted.session_epoch;
        let mut edits = PendingEdits::default();
        edits.begin(
            &runtime,
            0_u64,
            "generator-test",
            None,
            generator_apply_resolver(
                owner.clone(),
                candidate.clone(),
                BTreeSet::from(["keycap_width".into()]),
            ),
        );
        support::run_pending(&runtime).await;
        assert!(matches!(
            edits.settle(true).as_slice(),
            [PendingEditResult::Landed { .. }]
        ));
        let accepted = runtime.model().accepted.unwrap();
        let applied = &accepted.document.definitions[0];
        assert_eq!(applied.name, "Latest accepted name");
        assert_eq!(applied.mechanical_profile, latest.mechanical_profile);
        assert_eq!(applied.generator, candidate.generator);
        assert_eq!(applied.pads, candidate.pads);
        assert_eq!(applied.courtyard, candidate.courtyard);

        let mut deleted = accepted.document.as_ref().clone();
        deleted.definitions.clear();
        support::open_document(&runtime, deleted).await;
        owner.session_epoch = runtime.model().accepted.unwrap().session_epoch;
        edits.begin(
            &runtime,
            1_u64,
            "generator-test",
            None,
            generator_apply_resolver(owner, candidate, BTreeSet::from(["keycap_width".into()])),
        );
        support::run_pending(&runtime).await;
        assert!(matches!(
            edits.settle(true).as_slice(),
            [PendingEditResult::Failed { message, .. }] if message.contains("removed")
        ));
    }

    #[wasm_bindgen_test]
    async fn mounted_apply_queued_behind_rename_preserves_both_and_undo() {
        let (root, fixture, _) = mount_generator_editor().await;
        let initial = take_initial_default_request(&fixture.requests).await;
        finish_request(initial, Err("superseded by test edit".into())).await;
        dispatch_width(&root, "20");
        let request = take_request(&fixture.requests, 20.0).await;
        let prepared = candidate(&request);
        finish_request(request, Ok(prepared)).await;
        wait_for_text(
            &root,
            "[role='status']",
            "Current generator preview is ready",
        )
        .await;
        let (entered, release) = support::gate_next_core_reply(&fixture.runtime);
        let mut renames = PendingEdits::default();
        renames.begin(
            &fixture.runtime,
            0_u64,
            "definition-name",
            Some("name".into()),
            boardstudio_application::EditResolver::new(
                "test-definition-rename",
                |accepted: &AcceptedSnapshot| {
                    let mut document = accepted.document.as_ref().clone();
                    document.definitions[0].name = "Queued name".into();
                    crate::parts_custom_definition::replacement_commit(
                        EditOperation::ReplaceDocument {
                            document: Box::new(document),
                        },
                        vec![DEFINITION_ID.into()],
                    )
                },
            ),
        );
        support::drive_pending(&fixture.runtime);
        entered.await.unwrap();
        let apply = root
            .query_selector(".m1-generator-settings > button.m1-generator-apply")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        apply.click();
        gloo_timers::future::TimeoutFuture::new(30).await;
        support::drive_pending(&fixture.runtime);
        release.send(()).unwrap();
        for _ in 0..20 {
            support::run_pending(&fixture.runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let accepted = fixture.runtime.model().accepted.unwrap();
        let definition = accepted
            .document
            .definitions
            .iter()
            .find(|definition| definition.id == DEFINITION_ID)
            .unwrap();
        assert_eq!(
            definition.name, "Queued name",
            "queued Apply preserves the preceding rename"
        );
        assert_eq!(
            definition.generator.as_ref().unwrap().parameters["keycap_width"].as_f64(),
            Some(20.0)
        );
        fixture
            .runtime
            .submit(boardstudio_application::Event::Undo {
                operation_id: fixture.runtime.operation(),
            });
        support::run_pending(&fixture.runtime).await;
        let accepted = fixture.runtime.model().accepted.unwrap();
        let definition = accepted
            .document
            .definitions
            .iter()
            .find(|definition| definition.id == DEFINITION_ID)
            .unwrap();
        assert_eq!(definition.name, "Queued name");
        assert_eq!(
            definition.generator.as_ref().unwrap().parameters["keycap_width"].as_f64(),
            Some(18.0)
        );
        fixture
            .runtime
            .submit(boardstudio_application::Event::Undo {
                operation_id: fixture.runtime.operation(),
            });
        support::run_pending(&fixture.runtime).await;
        assert_eq!(
            fixture
                .runtime
                .model()
                .accepted
                .unwrap()
                .document
                .definitions[0]
                .name,
            fixture.first.name
        );
        fixture.runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn uploaded_model_projects_accepted_filename_without_erasing_dirty_width() {
        uploaded_model_projection(false).await;
    }

    #[wasm_bindgen_test]
    async fn upload_preview_refresh_does_not_keep_a_later_failed_apply_draft() {
        uploaded_model_projection(true).await;
    }

    async fn uploaded_model_projection(fail_queued_apply: bool) {
        let (root, fixture, handles) = mount_generator_editor().await;
        let initial = take_initial_default_request(&fixture.requests).await;
        finish_request(initial, Err("superseded".into())).await;
        dispatch_width(&root, "21");
        let request = take_request(&fixture.requests, 21.0).await;
        let prepared = candidate(&request);
        finish_request(request, Ok(prepared)).await;
        let field = root
            .query_selector("input[aria-label='switch_3dmodel_filename']")
            .unwrap()
            .unwrap()
            .parent_element()
            .unwrap()
            .parent_element()
            .unwrap();
        let input = field
            .query_selector("input[type=file]")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        let file = web_sys::File::new_with_str_sequence(
            &js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(
                "solid test\nendsolid test\n",
            )),
            "test.stl",
        )
        .unwrap();
        let descriptor = js_sys::Object::new();
        js_sys::Reflect::set(&descriptor, &"value".into(), &js_sys::Array::of1(&file)).unwrap();
        js_sys::Object::define_property(
            input.unchecked_ref::<js_sys::Object>(),
            &"files".into(),
            &descriptor,
        );
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &init).unwrap())
            .unwrap();
        let mut effects = VecDeque::new();
        let mut apply_queued = false;
        for _ in 0..100 {
            gloo_timers::future::TimeoutFuture::new(20).await;
            effects.extend(support::take_held_effects(&fixture.runtime));
            if fail_queued_apply && !apply_queued && !effects.is_empty() {
                root.query_selector(".m1-generator-settings > button.m1-generator-apply")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<web_sys::HtmlElement>()
                    .unwrap()
                    .click();
                apply_queued = true;
            }
            if let Some(effect) = effects.pop_front() {
                effects.extend(support::run_effect(&fixture.runtime, effect).await);
            }
            if fixture
                .runtime
                .model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .assets
                .iter()
                .any(|asset| asset.name == "test.stl")
            {
                break;
            }
        }
        let mut deferred_core = VecDeque::new();
        while let Some(effect) = effects.pop_front() {
            if matches!(effect, boardstudio_application::Effect::Core { .. }) {
                deferred_core.push_back(effect);
            } else {
                effects.extend(support::run_effect(&fixture.runtime, effect).await);
            }
        }
        effects = deferred_core;
        gloo_timers::future::TimeoutFuture::new(30).await;
        let accepted = fixture.runtime.model().accepted.unwrap();
        let filename = accepted.document.definitions[0]
            .generator
            .as_ref()
            .unwrap()
            .parameters["switch_3dmodel_filename"]
            .as_str()
            .unwrap();
        assert!(filename.starts_with("boardstudio-asset:"));
        let displayed = field
            .query_selector("input[type=text]")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        assert_eq!(displayed.value(), filename);
        assert!(field.text_content().unwrap().contains("Remove model"));
        assert_eq!(
            root.query_selector("input[aria-label='Keycap Width']")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap()
                .value(),
            "21"
        );
        let refreshed = take_request(&fixture.requests, 21.0).await;
        let prepared = candidate(&refreshed);
        assert!(
            boardstudio_core::generators::model_bindings(&prepared, None)
                .unwrap()
                .iter()
                .any(|model| filename == format!("boardstudio-asset:{}", model.asset_id))
        );
        if fail_queued_apply {
            assert!(apply_queued);
            support::fail_next_core_reply(&fixture.runtime, "Apply after upload failed");
        }
        effects.extend(support::take_held_effects(&fixture.runtime));
        while let Some(effect) = effects.pop_front() {
            effects.extend(support::run_effect(&fixture.runtime, effect).await);
        }
        if fail_queued_apply {
            wait_for_text(&root, "[role='alert']", "Apply after upload failed").await;
            assert_eq!(
                root.query_selector("input[aria-label='Keycap Width']")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value(),
                "18.0"
            );
        }
        finish_request(refreshed, Ok(prepared)).await;
        let store = handles.borrow().as_ref().unwrap().store;
        if fail_queued_apply {
            assert!(
                store.read().is_none(),
                "late preview cannot restore the rejected draft"
            );
        } else {
            assert!(store.read().as_ref().unwrap().definition.is_some());
        }
        fixture.runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn queued_apply_preserves_a_preceding_generator_model_upload() {
        let runtime = support::new_runtime();
        let base = definition();
        let mut doc = boardstudio_core::model::ProjectDoc::empty("upload-apply", "Upload Apply");
        doc.definitions.push(base.clone());
        support::open_document(&runtime, doc).await;
        let accepted = runtime.model().accepted.unwrap();
        let owner = make_owner(&accepted, runtime.scope(), None, &base, 1, 1).unwrap();
        let asset = Asset {
            id: String::new(),
            name: "Model".into(),
            media_type: "model/stl".into(),
            sha256: "a".repeat(64),
            license: None,
            source: None,
        };
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let mut edits = PendingEdits::default();
        edits.begin(
            &runtime,
            0_u64,
            "upload-test",
            None,
            generator_model_upload_resolver(
                owner.clone(),
                base.clone(),
                "switch_3dmodel_filename".into(),
                asset,
                71,
            ),
        );
        support::drive_pending(&runtime);
        entered.await.unwrap();
        let prepared = apply_input(
            &base,
            &boardstudio_core::generators::parameter_schema(&owner.source).unwrap(),
            &BTreeMap::from([("keycap_width".into(), Value::from(20))]),
        )
        .unwrap();
        edits.begin(
            &runtime,
            1_u64,
            "apply-test",
            None,
            generator_apply_resolver(owner, prepared, BTreeSet::from(["keycap_width".into()])),
        );
        release.send(()).unwrap();
        gloo_timers::future::TimeoutFuture::new(30).await;
        support::run_pending(&runtime).await;
        assert!(matches!(
            edits.settle(true).as_slice(),
            [
                PendingEditResult::Landed { .. },
                PendingEditResult::Landed { .. }
            ],
            "the queued apply runs behind the held upload and both land"
        ));
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            accepted.document.definitions[0]
                .generator
                .as_ref()
                .unwrap()
                .parameters
                .get("switch_3dmodel_filename"),
            Some(&Value::String(
                "boardstudio-asset:generator-model-71".into()
            ))
        );
        assert!(
            boardstudio_core::generators::model_bindings(&accepted.document.definitions[0], None)
                .unwrap()
                .iter()
                .any(|model| model.asset_id == "generator-model-71")
        );
    }

    #[wasm_bindgen_test]
    async fn a_new_apply_can_return_a_parameter_to_its_original_value() {
        let (root, fixture, _) = mount_generator_editor().await;
        let initial = take_initial_default_request(&fixture.requests).await;
        finish_request(initial, Err("superseded".into())).await;
        dispatch_width(&root, "20");
        let request = take_request(&fixture.requests, 20.0).await;
        let ready = candidate(&request);
        finish_request(request, Ok(ready)).await;
        wait_for_text(
            &root,
            "[role='status']",
            "Current generator preview is ready",
        )
        .await;
        let button = root
            .query_selector(".m1-generator-settings > button.m1-generator-apply")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        let (entered, release) = support::gate_next_core_reply(&fixture.runtime);
        button.click();
        gloo_timers::future::TimeoutFuture::new(30).await;
        support::drive_pending(&fixture.runtime);
        entered.await.unwrap();
        dispatch_width(&root, "18");
        let request = take_request(&fixture.requests, 18.0).await;
        let ready = candidate(&request);
        finish_request(request, Ok(ready)).await;
        wait_for_text(
            &root,
            "[role='status']",
            "Current generator preview is ready",
        )
        .await;
        release.send(()).unwrap();
        gloo_timers::future::TimeoutFuture::new(30).await;
        support::run_pending(&fixture.runtime).await;
        gloo_timers::future::TimeoutFuture::new(30).await;
        assert_eq!(
            fixture
                .runtime
                .model()
                .accepted
                .unwrap()
                .document
                .definitions[0]
                .generator
                .as_ref()
                .unwrap()
                .parameters["keycap_width"],
            Value::from(20.0)
        );
        button.click();
        gloo_timers::future::TimeoutFuture::new(30).await;
        support::run_pending(&fixture.runtime).await;
        gloo_timers::future::TimeoutFuture::new(30).await;
        let accepted = fixture.runtime.model().accepted.unwrap();
        assert_eq!(
            accepted.document.definitions[0]
                .generator
                .as_ref()
                .unwrap()
                .parameters["keycap_width"],
            Value::from(18.0)
        );
        let normalized = boardstudio_core::generators::normalize_definition(
            accepted.document.definitions[0].clone(),
        )
        .unwrap();
        assert_eq!(
            accepted.document.definitions[0].courtyard,
            normalized.courtyard
        );
        fixture.runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn failed_apply_restores_accepted_parameters_and_reports_failure() {
        let (root, fixture, _) = mount_generator_editor().await;
        let initial = take_initial_default_request(&fixture.requests).await;
        finish_request(initial, Err("superseded".into())).await;
        dispatch_width(&root, "20");
        let request = take_request(&fixture.requests, 20.0).await;
        let prepared = candidate(&request);
        finish_request(request, Ok(prepared)).await;
        wait_for_text(
            &root,
            "[role='status']",
            "Current generator preview is ready",
        )
        .await;
        support::fail_next_core_reply(&fixture.runtime, "controlled Apply failure");
        root.query_selector(".m1-generator-settings > button.m1-generator-apply")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        gloo_timers::future::TimeoutFuture::new(30).await;
        support::run_pending(&fixture.runtime).await;
        wait_for_text(&root, "[role='alert']", "controlled Apply failure").await;
        let input = root
            .query_selector("input[aria-label='Keycap Width']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        assert_eq!(input.value(), "18.0");
        fixture.runtime.unsubscribe();
        root.remove();
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
            18.0
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
        assert!(
            pad_one.at.x > 0.0,
            "Front preview places pad 1 on the right"
        );
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
