//! Pair and linked-feature validation.

use super::*;

/// Validates every mechanical mirror pair in the document.
///
/// Returns the first violation as a message naming the pair, instance or feature
/// it concerns. A document without hardware has no pairs and validates.
pub fn validate(document: &ProjectDoc) -> Result<(), String> {
    let Some(hardware) = document.hardware.as_ref() else {
        return Ok(());
    };
    let mut pair_ids: Vec<&str> = Vec::new();
    let mut owners: Vec<(&str, &str)> = Vec::new();
    for pair in &hardware.pairs {
        if pair.id.trim().is_empty() {
            return Err("a mechanical pair has an empty id".into());
        }
        if pair_ids.contains(&pair.id.as_str()) {
            return Err(format!("mechanical pair id {:?} is not unique", pair.id));
        }
        pair_ids.push(&pair.id);
        let [first, second] = &pair.members;
        if first == second {
            return Err(format!(
                "mechanical pair {:?} names the same physical instance twice",
                pair.id
            ));
        }
        for member in &pair.members {
            if !hardware
                .instances
                .iter()
                .any(|instance| instance.id == *member)
            {
                return Err(format!(
                    "mechanical pair {:?} names physical instance {member:?}, which is not in the document",
                    pair.id
                ));
            }
            if let Some((_, other)) = owners
                .iter()
                .find(|(instance, _)| instance == &member.as_str())
            {
                return Err(format!(
                    "physical instance {member:?} belongs to mechanical pairs {other:?} and {:?}",
                    pair.id
                ));
            }
            owners.push((member, &pair.id));
        }
        if !pair.mapping.at.x.is_finite()
            || !pair.mapping.at.y.is_finite()
            || !pair.mapping.rotation.is_finite()
        {
            return Err(format!(
                "mechanical pair {:?} has a non-finite mapping",
                pair.id
            ));
        }
        validate_features(pair, hardware)?;
    }
    super::super::halves::validate_document(document)
}

fn validate_features(
    pair: &MechanicalMirrorPair,
    hardware: &HardwareConfiguration,
) -> Result<(), String> {
    let mut feature_ids: Vec<&str> = Vec::new();
    let mut member_feature_ids: Vec<(&str, &str)> = Vec::new();
    for feature in &pair.features {
        if feature.id.trim().is_empty() {
            return Err(format!(
                "mechanical pair {:?} has a feature with an empty id",
                pair.id
            ));
        }
        if feature_ids.contains(&feature.id.as_str()) {
            return Err(format!(
                "mechanical pair {:?} has more than one feature {:?}",
                pair.id, feature.id
            ));
        }
        feature_ids.push(&feature.id);
        if !value_matches_kind(feature.kind, &feature.value) {
            return Err(format!(
                "mechanical feature {:?} of pair {:?} has kind {:?} but a mismatched value",
                feature.id, pair.id, feature.kind
            ));
        }
        if !pair.members.contains(&feature.author) {
            return Err(format!(
                "mechanical feature {:?} of pair {:?} authors from {:?}, which is not a pair member",
                feature.id, pair.id, feature.author
            ));
        }
        validate_feature_value(&feature.id, &feature.value)?;
        if feature.members.len() != 2 {
            return Err(format!(
                "mechanical feature {:?} of pair {:?} must have one record per pair member, found {}",
                feature.id,
                pair.id,
                feature.members.len()
            ));
        }
        for member in &feature.members {
            if !pair.members.contains(&member.instance_id) {
                return Err(format!(
                    "mechanical feature {:?} of pair {:?} records {:?}, which is not a pair member",
                    feature.id, pair.id, member.instance_id
                ));
            }
            if member.feature_id.trim().is_empty() {
                return Err(format!(
                    "mechanical feature {:?} of pair {:?} has an empty member feature id for {:?}",
                    feature.id, pair.id, member.instance_id
                ));
            }
            if member_feature_ids.iter().any(|(instance, id)| {
                *instance == member.instance_id.as_str() && *id == member.feature_id.as_str()
            }) {
                return Err(format!(
                    "member feature id {:?} is not unique for physical instance {:?} in pair {:?}",
                    member.feature_id, member.instance_id, pair.id
                ));
            }
            member_feature_ids.push((&member.instance_id, &member.feature_id));
        }
        for member in &pair.members {
            let count = feature
                .members
                .iter()
                .filter(|record| &record.instance_id == member)
                .count();
            if count != 1 {
                return Err(format!(
                    "mechanical feature {:?} of pair {:?} must have exactly one record for {member:?}",
                    feature.id, pair.id
                ));
            }
        }
        let author = feature
            .members
            .iter()
            .find(|record| record.instance_id == feature.author)
            .ok_or_else(|| {
                format!(
                    "mechanical feature {:?} of pair {:?} has no record for its author {:?}",
                    feature.id, pair.id, feature.author
                )
            })?;
        if author.state != MechanicalFeatureState::Linked {
            return Err(format!(
                "mechanical feature {:?} of pair {:?} is authored by {:?} but that record is {:?}; the stored value is the linked value",
                feature.id, pair.id, feature.author, author.state
            ));
        }
    }
    validate_construction(pair, hardware)
}

fn validate_construction(
    pair: &MechanicalMirrorPair,
    hardware: &HardwareConfiguration,
) -> Result<(), String> {
    for member_id in &pair.members {
        let instance = hardware
            .instances
            .iter()
            .find(|instance| &instance.id == member_id)
            .ok_or_else(|| {
                format!(
                    "mechanical pair {:?} names physical instance {member_id:?}, which is not in the document",
                    pair.id
                )
            })?;
        match instance.construction_ownership {
            MechanicalConstructionOwnership::Pair => {
                if instance.mechanical.is_none() {
                    return Err(format!(
                        "physical instance {member_id:?} of mechanical pair {:?} is pair-owned but has no stored mechanical configuration",
                        pair.id
                    ));
                }
            }
            MechanicalConstructionOwnership::Local => {
                let Some(configuration) = instance.mechanical.as_ref() else {
                    return Err(format!(
                        "physical instance {member_id:?} of mechanical pair {:?} owns its construction locally but has no stored mechanical configuration",
                        pair.id
                    ));
                };
                if !construction_matches(configuration, &pair.construction) {
                    return Err(format!(
                        "physical instance {member_id:?} owns its construction locally, but its shared subset differs from mechanical pair {:?}'s construction",
                        pair.id
                    ));
                }
            }
            MechanicalConstructionOwnership::Legacy => {}
        }
    }
    Ok(())
}

fn construction_matches(
    configuration: &MechanicalConfiguration,
    construction: &MechanicalConstruction,
) -> bool {
    let mut expected = construction.clone();
    if let Some(layout) = expected.gasket_layout.as_mut() {
        layout.supports.clear();
    }
    shared_construction(configuration) == expected
}

fn value_matches_kind(kind: MechanicalFeatureKind, value: &MechanicalFeatureValue) -> bool {
    matches!(
        (kind, value),
        (
            MechanicalFeatureKind::Mount | MechanicalFeatureKind::ClosureMount,
            MechanicalFeatureValue::Mount { .. }
        ) | (
            MechanicalFeatureKind::Opening,
            MechanicalFeatureValue::Opening { .. }
        ) | (
            MechanicalFeatureKind::GasketSupport,
            MechanicalFeatureValue::GasketSupport { .. }
        )
    )
}

fn validate_feature_value(feature_id: &str, value: &MechanicalFeatureValue) -> Result<(), String> {
    let finite = |label: &str, value: f64| {
        if value.is_finite() {
            Ok(())
        } else {
            Err(format!(
                "mechanical feature {feature_id:?} has a non-finite {label}"
            ))
        }
    };
    match value {
        MechanicalFeatureValue::Mount { mount } => {
            finite("mount point", mount.at.x)?;
            finite("mount point", mount.at.y)?;
            finite("mount hole diameter", mount.hole_diameter)?;
            if let Some(diameter) = mount.boss_diameter {
                finite("mount boss diameter", diameter)?;
            }
            if let Some(height) = mount.height {
                finite("mount height", height)?;
            }
        }
        MechanicalFeatureValue::Opening { opening } => {
            finite("opening z", opening.z)?;
            finite("opening height", opening.height)?;
            for vertex in &opening.vertices {
                finite("opening vertex", vertex.point.x)?;
                finite("opening vertex", vertex.point.y)?;
            }
        }
        MechanicalFeatureValue::GasketSupport { anchor } => {
            finite("gasket anchor", anchor.anchor)?;
            if let Some(length) = anchor.length {
                finite("gasket support length", length)?;
            }
            if let Some(width) = anchor.width {
                finite("gasket support width", width)?;
            }
        }
    }
    Ok(())
}
