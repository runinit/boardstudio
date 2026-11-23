//! Linked-feature lifecycle: local, omitted and rejoined pair feature members.

use super::super::change::{MechanicalChange, MechanicalEdit, MechanicalTarget};
use super::super::features;
use super::super::inputs::effective_inputs;
use super::validation::upsert_item;
use super::{dedup_changed, project};
use crate::model::*;

/// Applies one per-feature lifecycle action and projects the result.
pub(super) fn apply_feature_lifecycle(
    doc: &mut ProjectDoc,
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    let changed = match &edit.change {
        MechanicalChange::MakeFeatureLocal {
            feature_id,
            instance_id,
        } => make_feature_local(doc, feature_id, instance_id)?,
        MechanicalChange::OmitFeatureMember {
            feature_id,
            instance_id,
        } => omit_feature_member(doc, feature_id, instance_id)?,
        MechanicalChange::RejoinFeatureMember {
            feature_id,
            instance_id,
            source_instance_id,
        } => rejoin_feature_member(doc, feature_id, instance_id, source_instance_id)?,
        _ => return Err("The feature lifecycle was dispatched with another change.".into()),
    };
    if changed.is_empty() {
        return Ok(changed);
    }
    let mut changed = changed;
    project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Where a linked feature lives and the state of one half.
pub(super) struct PairFeatureLocation {
    pub(super) pair_index: usize,
    pub(super) feature_index: usize,
    pub(super) state: MechanicalFeatureState,
}

pub(super) fn locate_pair_feature(
    doc: &ProjectDoc,
    instance_id: &str,
    feature_id: &str,
) -> Result<PairFeatureLocation, String> {
    let hardware = doc
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?;
    let pair_index = hardware
        .pairs
        .iter()
        .position(|pair| features::member_state(pair, instance_id, feature_id).is_some())
        .ok_or_else(|| {
            format!("Physical instance {instance_id:?} has no linked feature {feature_id:?}.")
        })?;
    let pair = &hardware.pairs[pair_index];
    let feature_index = pair
        .features
        .iter()
        .position(|feature| feature_member_id(feature, instance_id) == Some(feature_id))
        .ok_or_else(|| {
            format!(
                "Mechanical pair {:?} has no feature {feature_id:?}.",
                pair.id
            )
        })?;
    let state = features::member_state(pair, instance_id, feature_id)
        .ok_or("Mechanical feature state is unavailable")?;
    Ok(PairFeatureLocation {
        pair_index,
        feature_index,
        state,
    })
}

/// Materializes one half's visible geometry and makes it local.
///
/// A half that is already local is unchanged. The other half and every other
/// feature keep their state; the copied geometry replaces the same member
/// feature id in the half's ordinary configuration, so nothing moves. When the
/// selected half is the author, authorship moves to the other linked half with
/// the stored value re-expressed in that half's frame, so the pair keeps exactly
/// one linked author and the other half still derives the same geometry.
pub(super) fn make_feature_local(
    doc: &mut ProjectDoc,
    feature_id: &str,
    instance_id: &str,
) -> Result<Vec<String>, String> {
    let location = locate_pair_feature(doc, instance_id, feature_id)?;
    match location.state {
        MechanicalFeatureState::Local => return Ok(Vec::new()),
        MechanicalFeatureState::Omitted => {
            return Err(format!(
                "Feature {feature_id:?} is omitted on {instance_id:?}; rejoin it before making it local."
            ));
        }
        MechanicalFeatureState::Linked => {}
    }
    let (kind, visible) = {
        let pair = doc.hardware.as_ref().unwrap().pairs[location.pair_index].clone();
        let feature = &pair.features[location.feature_index];
        (
            feature.kind,
            feature_visible_value(&pair, feature, instance_id)?,
        )
    };
    if let Some(value) = visible {
        let configuration = instance_configuration_mut(doc, instance_id)?;
        install_feature(configuration, kind, feature_id, &value)?;
    }
    transfer_authorship(doc, &location, instance_id)?;
    features::make_local(
        &mut doc.hardware.as_mut().unwrap().pairs[location.pair_index],
        instance_id,
        feature_id,
    )?;
    let mut changed = vec![feature_id.to_string(), instance_id.to_string()];
    changed.extend(pair_owner_ids(doc, location.pair_index));
    Ok(changed)
}

/// Removes one half's geometry and records a local omission.
///
/// The other half and every other feature keep their state. An already omitted
/// half is unchanged. When the selected half is the author, authorship moves to
/// the other linked half first, so the surviving half still derives the same
/// geometry with no second authored value.
pub(super) fn omit_feature_member(
    doc: &mut ProjectDoc,
    feature_id: &str,
    instance_id: &str,
) -> Result<Vec<String>, String> {
    let location = locate_pair_feature(doc, instance_id, feature_id)?;
    if location.state == MechanicalFeatureState::Omitted {
        return Ok(Vec::new());
    }
    let kind = doc.hardware.as_ref().unwrap().pairs[location.pair_index].features
        [location.feature_index]
        .kind;
    remove_feature(
        instance_configuration_mut(doc, instance_id)?,
        kind,
        feature_id,
    );
    transfer_authorship(doc, &location, instance_id)?;
    features::omit_member(
        &mut doc.hardware.as_mut().unwrap().pairs[location.pair_index],
        instance_id,
        feature_id,
    )?;
    let mut changed = vec![feature_id.to_string(), instance_id.to_string()];
    changed.extend(pair_owner_ids(doc, location.pair_index));
    Ok(changed)
}

/// Moves a feature's authorship off `instance_id` when that half was the author.
///
/// The stored value is rewritten in the other linked half's frame, so that half
/// derives exactly the geometry it showed before and the pair keeps its single
/// authored value. A no-op when `instance_id` is not the author.
pub(super) fn transfer_authorship(
    doc: &mut ProjectDoc,
    location: &PairFeatureLocation,
    instance_id: &str,
) -> Result<(), String> {
    let pair = doc.hardware.as_ref().unwrap().pairs[location.pair_index].clone();
    let feature = &pair.features[location.feature_index];
    if feature.author != instance_id {
        return Ok(());
    }
    let other = pair
        .members
        .iter()
        .find(|member| member.as_str() != instance_id)
        .cloned()
        .ok_or("The mechanical pair has no other member.")?;
    let other_visible = feature_visible_value(&pair, feature, &other)?.ok_or_else(|| {
        format!("Physical instance {other:?} has no linked geometry to author the feature.")
    })?;
    let pair = &mut doc.hardware.as_mut().unwrap().pairs[location.pair_index];
    let feature = &mut pair.features[location.feature_index];
    feature.value = other_visible;
    feature.author = other;
    Ok(())
}

/// Clears one half's exception, adopting the chosen source's geometry.
///
/// The chosen source's visible (or local) geometry becomes the shared authored
/// value, the counterpart is derived from it through the pair map, and that
/// reflected geometry is installed on the rejoined half so it shows the reviewed
/// result immediately. The other half and every other feature keep their state.
pub(super) fn rejoin_feature_member(
    doc: &mut ProjectDoc,
    feature_id: &str,
    instance_id: &str,
    source_instance_id: &str,
) -> Result<Vec<String>, String> {
    let location = locate_pair_feature(doc, instance_id, feature_id)?;
    if location.state == MechanicalFeatureState::Linked {
        return Ok(Vec::new());
    }
    let pair = doc.hardware.as_ref().unwrap().pairs[location.pair_index].clone();
    if !pair
        .members
        .iter()
        .any(|member| member == source_instance_id)
    {
        return Err(format!(
            "Physical instance {source_instance_id:?} is not a member of mechanical pair {:?}.",
            pair.id
        ));
    }
    // The selected half may deliberately choose its own current geometry as the source,
    // so the reviewed result can keep that half's placement and re-derive the other.
    let kind = pair.features[location.feature_index].kind;
    let source_state =
        features::member_state(&pair, source_instance_id, feature_id).ok_or_else(|| {
            format!(
                "Physical instance {source_instance_id:?} has no linked feature {feature_id:?}."
            )
        })?;
    let source_value = match source_state {
        MechanicalFeatureState::Omitted => {
            return Err(format!(
                "Physical instance {source_instance_id:?} has no geometry for feature {feature_id:?}."
            ));
        }
        MechanicalFeatureState::Linked => {
            feature_visible_value(&pair, &pair.features[location.feature_index], source_instance_id)?
                .ok_or_else(|| {
                    format!(
                        "Physical instance {source_instance_id:?} has no visible geometry for feature {feature_id:?}."
                    )
                })?
        }
        MechanicalFeatureState::Local => find_feature_value(
            &effective_inputs(
                doc,
                &[],
                &MechanicalTarget::Instance {
                    instance_id: source_instance_id.into(),
                },
            )?
            .configuration,
            kind,
            feature_id,
        )
        .ok_or_else(|| {
            format!(
                "Physical instance {source_instance_id:?} has no local geometry for feature {feature_id:?}."
            )
        })?,
    };
    {
        let pair = &mut doc.hardware.as_mut().unwrap().pairs[location.pair_index];
        let feature = &mut pair.features[location.feature_index];
        feature.value = source_value;
        feature.author = source_instance_id.to_string();
        features::rejoin_member(pair, instance_id, feature_id)?;
    }
    let derived = {
        let pair = &doc.hardware.as_ref().unwrap().pairs[location.pair_index];
        let feature = &pair.features[location.feature_index];
        if let Some(derived) = features::counterpart(pair, feature, source_instance_id)? {
            Some(derived)
        } else {
            // The chosen source may itself be a local exception. Its geometry is
            // now the shared authored value, so derive the counterpart from a
            // linked view without clearing the source's own exception.
            let mut probe = pair.clone();
            for member in &mut probe.features[location.feature_index].members {
                member.state = MechanicalFeatureState::Linked;
            }
            features::counterpart(
                &probe,
                &probe.features[location.feature_index],
                source_instance_id,
            )?
        }
    };
    // The pair is the single source for this feature again, so the half that is not the
    // source derives its reflected geometry on read; no copy is written back to a local
    // configuration. The derivation is still checked, so a source that cannot derive the
    // counterpart half is rejected. Choosing the selected half's own geometry makes the
    // other half the one that derives.
    let expected_counterpart = if source_instance_id == instance_id {
        pair.members
            .iter()
            .find(|member| *member != instance_id)
            .cloned()
    } else {
        Some(instance_id.to_string())
    };
    if let Some(derived) = derived
        && expected_counterpart.as_deref() != Some(derived.instance_id.as_str())
    {
        return Err("The rejoin source did not derive the counterpart half.".into());
    }
    // The pair owns the rejoined feature again, so the half's local copy must go:
    // a `Linked` member stores no geometry of its own.
    let member_feature_id = {
        let pair = &doc.hardware.as_ref().unwrap().pairs[location.pair_index];
        feature_member_id(&pair.features[location.feature_index], instance_id)
            .map(str::to_string)
            .ok_or_else(|| {
                format!(
                    "Physical instance {instance_id:?} has no record for feature {feature_id:?}."
                )
            })?
    };
    let configuration = instance_configuration_mut(doc, instance_id)?;
    remove_feature(configuration, kind, &member_feature_id);
    let mut changed = vec![
        feature_id.to_string(),
        instance_id.to_string(),
        source_instance_id.to_string(),
    ];
    changed.extend(pair_owner_ids(doc, location.pair_index));
    Ok(changed)
}

pub(super) fn feature_member_id<'a>(
    feature: &'a MechanicalFeature,
    instance_id: &str,
) -> Option<&'a str> {
    feature
        .members
        .iter()
        .find(|member| member.instance_id == instance_id)
        .map(|member| member.feature_id.as_str())
}

/// One half's visible geometry for a linked feature, or `None` when that half
/// owns its geometry locally or is omitted.
pub(super) fn feature_visible_value(
    pair: &MechanicalMirrorPair,
    feature: &MechanicalFeature,
    instance_id: &str,
) -> Result<Option<MechanicalFeatureValue>, String> {
    let member = feature
        .members
        .iter()
        .find(|member| member.instance_id == instance_id)
        .ok_or_else(|| {
            format!(
                "Mechanical feature {:?} has no record for {instance_id:?}.",
                feature.id
            )
        })?;
    match member.state {
        MechanicalFeatureState::Linked => {
            if feature.author == instance_id {
                Ok(Some(feature.value.clone()))
            } else {
                Ok(features::counterpart(pair, feature, &feature.author)?
                    .map(|derived| derived.value))
            }
        }
        MechanicalFeatureState::Local | MechanicalFeatureState::Omitted => Ok(None),
    }
}

/// The pair and both member instances a pair-record change touched.
pub(super) fn pair_owner_ids(doc: &ProjectDoc, pair_index: usize) -> Vec<String> {
    let pair = &doc.hardware.as_ref().unwrap().pairs[pair_index];
    let mut ids = vec![pair.id.clone()];
    ids.extend(pair.members.iter().cloned());
    ids
}

/// The durable ids one configuration carries for a feature kind.
pub(super) fn feature_ids(
    configuration: &MechanicalConfiguration,
    kind: MechanicalFeatureKind,
) -> std::collections::BTreeSet<String> {
    let ids = match kind {
        MechanicalFeatureKind::Mount => configuration
            .mounts
            .iter()
            .map(|mount| mount.id.clone())
            .collect::<Vec<_>>(),
        MechanicalFeatureKind::ClosureMount => configuration
            .closure_mounts
            .iter()
            .flatten()
            .map(|mount| mount.id.clone())
            .collect(),
        MechanicalFeatureKind::Opening => configuration
            .openings
            .iter()
            .flatten()
            .map(|opening| opening.id.clone())
            .collect(),
        MechanicalFeatureKind::GasketSupport => configuration
            .gasket_layout
            .iter()
            .flat_map(|layout| layout.supports.iter())
            .map(|anchor| anchor.id.clone())
            .collect(),
    };
    ids.into_iter().collect()
}

pub(super) fn find_feature_value(
    configuration: &MechanicalConfiguration,
    kind: MechanicalFeatureKind,
    feature_id: &str,
) -> Option<MechanicalFeatureValue> {
    match kind {
        MechanicalFeatureKind::Mount => configuration
            .mounts
            .iter()
            .find(|mount| mount.id == feature_id)
            .cloned()
            .map(|mount| MechanicalFeatureValue::Mount { mount }),
        MechanicalFeatureKind::ClosureMount => configuration
            .closure_mounts
            .as_ref()?
            .iter()
            .find(|mount| mount.id == feature_id)
            .cloned()
            .map(|mount| MechanicalFeatureValue::Mount { mount }),
        MechanicalFeatureKind::Opening => configuration
            .openings
            .as_ref()?
            .iter()
            .find(|opening| opening.id == feature_id)
            .cloned()
            .map(|opening| MechanicalFeatureValue::Opening { opening }),
        MechanicalFeatureKind::GasketSupport => configuration
            .gasket_layout
            .as_ref()?
            .supports
            .iter()
            .find(|anchor| anchor.id == feature_id)
            .cloned()
            .map(|anchor| MechanicalFeatureValue::GasketSupport { anchor }),
    }
}

/// Installs visible geometry under the destination member's durable identity.
/// The pair's authored value can carry a different member's id; vertex and anchor
/// associations remain part of the value and are preserved.
pub fn install_feature(
    configuration: &mut MechanicalConfiguration,
    kind: MechanicalFeatureKind,
    feature_id: &str,
    value: &MechanicalFeatureValue,
) -> Result<(), String> {
    match (kind, value) {
        (
            MechanicalFeatureKind::Mount | MechanicalFeatureKind::ClosureMount,
            MechanicalFeatureValue::Mount { mount },
        ) => {
            let mounts = if kind == MechanicalFeatureKind::Mount {
                &mut configuration.mounts
            } else {
                configuration.closure_mounts.get_or_insert_with(Vec::new)
            };
            let mut mount = mount.clone();
            mount.id = feature_id.to_owned();
            upsert_item(mounts, mount, |entry| &entry.id);
        }
        (MechanicalFeatureKind::Opening, MechanicalFeatureValue::Opening { opening }) => {
            let mut opening = opening.clone();
            opening.id = feature_id.to_owned();
            let openings = configuration.openings.get_or_insert_with(Vec::new);
            upsert_item(openings, opening, |entry| &entry.id);
        }
        (
            MechanicalFeatureKind::GasketSupport,
            MechanicalFeatureValue::GasketSupport { anchor },
        ) => {
            let mut anchor = anchor.clone();
            anchor.id = feature_id.to_owned();
            let layout = configuration
                .gasket_layout
                .get_or_insert_with(crate::mechanical::default_gasket_layout);
            upsert_item(&mut layout.supports, anchor, |entry| &entry.id);
        }
        _ => {
            return Err(format!(
                "Mechanical feature {feature_id:?} has a value that does not match its kind."
            ));
        }
    }
    Ok(())
}

pub fn remove_feature(
    configuration: &mut MechanicalConfiguration,
    kind: MechanicalFeatureKind,
    feature_id: &str,
) {
    match kind {
        MechanicalFeatureKind::Mount => {
            configuration.mounts.retain(|mount| mount.id != feature_id);
        }
        MechanicalFeatureKind::ClosureMount => {
            if let Some(mounts) = configuration.closure_mounts.as_mut() {
                mounts.retain(|mount| mount.id != feature_id);
            }
        }
        MechanicalFeatureKind::Opening => {
            if let Some(openings) = configuration.openings.as_mut() {
                openings.retain(|opening| opening.id != feature_id);
            }
        }
        MechanicalFeatureKind::GasketSupport => {
            if let Some(layout) = configuration.gasket_layout.as_mut() {
                layout.supports.retain(|anchor| anchor.id != feature_id);
            }
        }
    }
}

/// The mutable stored configuration of one physical instance.
pub(super) fn instance_configuration_mut<'a>(
    doc: &'a mut ProjectDoc,
    instance_id: &str,
) -> Result<&'a mut MechanicalConfiguration, String> {
    let hardware = doc
        .hardware
        .as_mut()
        .ok_or("Physical instances are unavailable")?;
    let instance = hardware
        .instances
        .iter_mut()
        .find(|instance| instance.id == instance_id)
        .ok_or_else(|| format!("Physical instance {instance_id:?} is unavailable."))?;
    instance.mechanical.as_mut().ok_or_else(|| {
        format!("Physical instance {instance_id:?} has no stored mechanical configuration.")
    })
}
