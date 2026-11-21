//! Accepted mechanical pair findings for a board or physical half.

use super::*;

/// The accepted mechanical pair findings that concern one board.
/// The accepted pair findings that concern one target.
///
/// A reading for one physical half sees a finding that names that half, or a finding
/// that names no half of the board at all (a board- or body-level problem). A finding
/// that names a *different* half of the same board is not this half's problem: two
/// halves sharing one PCB keep independent readiness, so a Case conflict on one half
/// cannot block the other's output.
pub(super) fn scoped_pair_findings(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    target: &MechanicalTarget,
) -> Vec<Finding> {
    let (board_id, selected_instance) = match target {
        MechanicalTarget::Board { board_id } => (board_id.clone(), None),
        MechanicalTarget::Instance { instance_id } => {
            let Some(instance) = document.hardware.as_ref().and_then(|hardware| {
                hardware
                    .instances
                    .iter()
                    .find(|instance| instance.id == *instance_id)
            }) else {
                return Vec::new();
            };
            (instance.board_id.clone(), Some(instance_id.clone()))
        }
    };
    let siblings: Vec<&str> = document
        .hardware
        .as_ref()
        .into_iter()
        .flat_map(|hardware| hardware.instances.iter())
        .filter(|instance| instance.board_id == board_id)
        .map(|instance| instance.id.as_str())
        .collect();
    pair_findings(document, board_contours)
        .into_iter()
        .filter(|finding| {
            if !finding.target_ids.contains(&board_id) {
                return false;
            }
            match &selected_instance {
                None => true,
                Some(instance_id) => {
                    finding.target_ids.iter().any(|id| id == instance_id)
                        || !finding
                            .target_ids
                            .iter()
                            .any(|id| siblings.iter().any(|sibling| sibling == id))
                }
            }
        })
        .collect()
}

/// Fit findings recomputed from the accepted pair state.
///
/// For every `Linked` feature the visible geometry each half shows is checked against
/// that half's accepted outline and against the features that half owns locally. A
/// feature whose reflected placement leaves the destination board, or lands on a
/// local feature, is retained exactly as placed - never relocated, snapped or
/// unlinked - and reported with the pair, instance, feature and board identities so
/// the user can find it. A linked closure mount outside its board is a fabricated-drill
/// violation (`Pcb`), so it also blocks that board's manufacturing export; a
/// Case-only clash leaves every board's PCB export alone.
pub fn pair_findings(document: &ProjectDoc, board_contours: &[BoardContours]) -> Vec<Finding> {
    let Some(hardware) = document.hardware.as_ref() else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    for pair in &hardware.pairs {
        for feature in &pair.features {
            for member in &feature.members {
                if member.state != MechanicalFeatureState::Linked {
                    continue;
                }
                let Some(instance) = hardware
                    .instances
                    .iter()
                    .find(|instance| instance.id == member.instance_id)
                else {
                    continue;
                };
                let Some(value) = visible_linked_value(pair, feature, &member.instance_id) else {
                    continue;
                };
                let contours = destination_contours(board_contours, instance);
                // A gasket anchor is resolved on the author's contours first, so its
                // placement finding is stated by that resolution, not from the raw
                // fraction here.
                let Some(point) = value_point(&value) else {
                    continue;
                };
                let outside =
                    !contours.is_empty() && !super::super::inside_outline(point, &contours);
                let conflicts = local_conflict(&value, instance, pair);
                let subject = match feature.kind {
                    MechanicalFeatureKind::Mount => "mount",
                    MechanicalFeatureKind::ClosureMount => "closure mount",
                    MechanicalFeatureKind::Opening => "opening",
                    MechanicalFeatureKind::GasketSupport => "gasket support",
                };
                if outside {
                    let scope = if feature.kind == MechanicalFeatureKind::ClosureMount {
                        Scope::Pcb
                    } else {
                        Scope::Case
                    };
                    // The refusal says the feature is linked and which half authored it, so the
                    // user knows where to reflect the hardware or make an exception.
                    let author = hardware
                        .instances
                        .iter()
                        .find(|entry| entry.id == feature.author)
                        .map_or(feature.author.as_str(), |entry| entry.name.as_str());
                    findings.push(pair_finding(
                        pair,
                        instance,
                        feature,
                        &member.instance_id,
                        scope,
                        format!(
                            "The linked {subject} at ({:.2}, {:.2}), authored by \"{author}\", lies outside this half's board outline.",
                            point.x, point.y
                        ),
                    ));
                } else if let Some(other) = conflicts {
                    findings.push(pair_finding(
                        pair,
                        instance,
                        feature,
                        &member.instance_id,
                        Scope::Case,
                        format!(
                            "The linked {subject} at ({:.2}, {:.2}) overlaps this half's local feature {other}.",
                            point.x, point.y
                        ),
                    ));
                }
            }
        }
    }
    findings
}

pub(super) fn pair_finding(
    pair: &MechanicalMirrorPair,
    instance: &PhysicalBoardInstance,
    feature: &MechanicalFeature,
    instance_id: &str,
    scope: Scope,
    message: String,
) -> Finding {
    Finding {
        id: format!(
            "mechanical:pair:{}:{}:{}:fit",
            pair.id, instance_id, feature.id
        ),
        severity: Severity::Error,
        scope,
        message,
        target_ids: vec![
            instance.board_id.clone(),
            instance_id.to_string(),
            feature.id.clone(),
        ],
    }
}

/// The destination half's accepted contours, mirrored for a flipped instance.
pub(super) fn destination_contours(
    board_contours: &[BoardContours],
    instance: &PhysicalBoardInstance,
) -> Vec<Contour> {
    let declared = declared_contours(board_contours, &instance.board_id);
    if instance.flipped {
        mirror_contours(declared)
    } else {
        declared
    }
}

/// One half's visible geometry for a linked feature, or `None` when it is not linked.
pub(super) fn visible_linked_value(
    pair: &MechanicalMirrorPair,
    feature: &MechanicalFeature,
    instance_id: &str,
) -> Option<MechanicalFeatureValue> {
    if feature.author == instance_id {
        return Some(feature.value.clone());
    }
    super::super::features::counterpart(pair, feature, &feature.author)
        .ok()
        .flatten()
        .map(|derived| derived.value)
}

/// A linked feature's representative point in its destination frame.
///
/// `None` for a gasket support, whose placement only exists once its anchor has been
/// resolved on the author's contours.
pub(super) fn value_point(value: &MechanicalFeatureValue) -> Option<Vec2> {
    match value {
        MechanicalFeatureValue::Mount { mount } => Some(mount.at),
        MechanicalFeatureValue::Opening { opening } => {
            let count = opening.vertices.len().max(1) as f64;
            Some(
                opening
                    .vertices
                    .iter()
                    .fold(Vec2 { x: 0.0, y: 0.0 }, |sum, vertex| Vec2 {
                        x: sum.x + vertex.point.x / count,
                        y: sum.y + vertex.point.y / count,
                    }),
            )
        }
        MechanicalFeatureValue::GasketSupport { .. } => None,
    }
}

/// A linked mount/closure mount's radius, or zero for other feature kinds.
pub(super) fn value_radius(value: &MechanicalFeatureValue) -> f64 {
    match value {
        MechanicalFeatureValue::Mount { mount } => {
            mount.boss_diameter.unwrap_or(mount.hole_diameter) / 2.0
        }
        _ => 0.0,
    }
}

/// The local feature a linked mount overlaps on its destination half, if any.
///
/// Only features that half owns locally are considered; a feature id belonging to the
/// pair is never treated as local hardware, so a link cannot clash with itself.
pub(super) fn local_conflict(
    value: &MechanicalFeatureValue,
    instance: &PhysicalBoardInstance,
    pair: &MechanicalMirrorPair,
) -> Option<String> {
    let MechanicalFeatureValue::Mount { mount } = value else {
        return None;
    };
    let owned: std::collections::BTreeSet<&str> = pair
        .features
        .iter()
        .flat_map(|feature| feature.members.iter())
        .filter(|member| member.instance_id == instance.id)
        .map(|member| member.feature_id.as_str())
        .collect();
    let configuration = instance.mechanical.as_ref()?;
    let radius = value_radius(value);
    let candidates = configuration
        .mounts
        .iter()
        .chain(configuration.closure_mounts.iter().flatten());
    for other in candidates {
        if owned.contains(other.id.as_str()) {
            continue;
        }
        let other_radius = other.boss_diameter.unwrap_or(other.hole_diameter) / 2.0;
        let distance = (mount.at.x - other.at.x).hypot(mount.at.y - other.at.y);
        if distance < radius + other_radius {
            return Some(other.id.clone());
        }
    }
    None
}
