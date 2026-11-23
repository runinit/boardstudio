use super::GeneratorOwner;
use crate::parts_custom_definition::replacement_commit;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_model::model::{Asset, EditOperation, Net, PartDefinition, Pin};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn remap_terminal_nets(
    document: &boardstudio_model::model::ProjectDoc,
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
            if accepted.document.id != owner.guard.document_id
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
            if accepted.document.id != owner.guard.document_id
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
    document: &boardstudio_model::model::ProjectDoc,
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
    let rebased = boardstudio_core_kernel::generators::normalize_definition(rebased)?;
    let nets = remap_terminal_nets(document, &original, &rebased)?;
    let mut replacement = document.clone();
    crate::parts_custom_definition::upsert_definition(&mut replacement, rebased.clone());
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

fn unique_generator_model_id(document: &boardstudio_model::model::ProjectDoc, seed: u64) -> String {
    crate::parts_custom_definition::unique_suffixed_id(
        &format!("generator-model-{seed}"),
        |candidate| document.assets.iter().any(|asset| asset.id == candidate),
    )
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
