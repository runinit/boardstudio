//! Core effective mechanical inputs.
//!
//! The single read path that resolves the configuration a board or physical half
//! actually uses: canonical settings, the legacy shared overlay with
//! first-instance fallback, instance settings, board thickness, wireless battery
//! defaults and flipped-instance handling. Apply, evaluation, findings and
//! presentation all reuse it. Filled in by the effective-inputs slice.
use crate::geometry::board_contours;
use crate::model::*;

use super::change::MechanicalTarget;

/// Fields a shared construction supplies over an instance's own configuration.
///
/// Authored openings, mounts, closure mounts, batteries, stabilizers, gasket
/// supports and board identity stay local to the instance.
const SHARED_OVERLAY_FIELDS: [&str; 20] = [
    "method",
    "mount",
    "integratedPlateFrame",
    "bottomStyle",
    "middleFrame",
    "plateThickness",
    "plateFoamThickness",
    "bottomFoamThickness",
    "bottomThickness",
    "plateToPcb",
    "wallThickness",
    "clearance",
    "gasket",
    "gasketTravel",
    "openingAllowance",
    "partProcesses",
    "internalGasket",
    "hardware",
    "criticalFits",
    "profiles",
];

/// The resolved mechanical configuration a target actually uses.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalInputs {
    pub board_id: String,
    pub configuration: MechanicalConfiguration,
    /// The target board's effective parts, mirrored in X and side-swapped for a
    /// flipped physical half, so a resolver reads the same geometry the scene carried.
    pub parts: Vec<Part>,
    /// The target's accepted board contours, mirrored in X for a flipped physical
    /// half, so a resolver reads the same accepted geometry the scene carried.
    pub board_contours: Vec<Contour>,
    /// Mechanical findings recomputed from accepted state that concern this target.
    /// The geometry they name is retained as placed; an affected assembly's output is
    /// what blocks, and unrelated boards are unaffected.
    pub findings: Vec<Finding>,
    /// Pair-linked gasket supports as this target sees them: each one resolved through
    /// the gasket resolver against its *author's* accepted contours and then reflected
    /// into this target's frame (placement, tangent and normal), never re-resolved from
    /// the author's perimeter fraction. `fit_error` is set when the reflected placement
    /// is not associated with this half's accepted outline; the intended placement is
    /// still reported, never snapped or moved.
    pub gasket_supports: Vec<MechanicalGasketSupport>,
}

impl MechanicalInputs {
    /// Whether this target's own accepted findings leave its Case output clear.
    ///
    /// The findings are already scoped to this target, so a Case-scope error here is this
    /// target's own problem; a conflict that names only a sibling half of the same board
    /// is not included. Adapters ask this instead of re-implementing the filter.
    pub fn case_findings_are_clear(&self) -> bool {
        !self.findings.iter().any(|finding| {
            matches!(finding.severity, Severity::Error) && matches!(finding.scope, Scope::Case)
        })
    }
}

/// Resolves the effective mechanical configuration for a board or physical half.
///
/// This is the single read path apply, evaluation and presentation share. A board
/// that exists but has no stored canonical configuration resolves to the Core-owned
/// defaults, so the adapter no longer needs its own `initial_settings` builder before
/// Enable. A missing board or physical instance is an error; the mechanical evaluator
/// retires such queued edits before they reach here.
///
/// For a member of an explicit mechanical pair the visible mounts, closure mounts and
/// authored openings are derived from the pair's single authored values here, so a
/// linked feature is never stored as a second writable copy on the member.
/// `board_contours` is the accepted outline projection this read is based on.
pub fn effective_inputs(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    target: &MechanicalTarget,
) -> Result<MechanicalInputs, String> {
    effective_inputs_inner(document, board_contours, target, true)
}

/// Resolves the effective inputs. `resolve_gaskets` is false while resolving an author's
/// own assembly, which keeps the gasket read path from recursing into itself.
fn effective_inputs_inner(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    target: &MechanicalTarget,
    resolve_gaskets: bool,
) -> Result<MechanicalInputs, String> {
    match target {
        MechanicalTarget::Board { board_id } => {
            if !document.boards.iter().any(|board| board.id == *board_id) {
                return Err(format!("Board {board_id:?} is unavailable"));
            }
            let stored = document
                .mechanical
                .as_ref()
                .filter(|configuration| configuration.board_id == *board_id)
                .cloned();
            let configuration = match stored {
                Some(configuration) => configuration,
                None => super::edit::default_configuration(document, board_id)?,
            };
            let configuration = apply_mechanical_defaults(document, configuration, board_id, false);
            let contours = declared_contours(board_contours, board_id);
            Ok(MechanicalInputs {
                board_id: board_id.clone(),
                configuration,
                parts: effective_parts(document, board_id, false),
                board_contours: contours,
                findings: scoped_pair_findings(document, board_contours, target),
                gasket_supports: Vec::new(),
            })
        }
        MechanicalTarget::Instance { instance_id } => {
            let hardware = document.hardware.as_ref().ok_or_else(|| {
                format!("physical instance {instance_id:?} is unavailable: no physical hardware")
            })?;
            let instance = hardware
                .instances
                .iter()
                .find(|instance| instance.id == *instance_id)
                .ok_or_else(|| format!("physical instance {instance_id:?} is unavailable"))?;
            let thickness = board_thickness(document, &instance.board_id)?;
            let configuration = resolve_instance_configuration(hardware, instance, thickness)?;
            let configuration = project_linked_features(hardware, instance, configuration)?;
            let configuration = apply_mechanical_defaults(
                document,
                configuration,
                &instance.board_id,
                instance.flipped,
            );
            let contours = declared_contours(board_contours, &instance.board_id);
            Ok(MechanicalInputs {
                board_id: instance.board_id.clone(),
                gasket_supports: if resolve_gaskets {
                    resolved_pair_gasket_supports(document, board_contours, target, &configuration)?
                } else {
                    Vec::new()
                },
                configuration,
                parts: effective_parts(document, &instance.board_id, instance.flipped),
                board_contours: if instance.flipped {
                    mirror_contours(contours)
                } else {
                    contours
                },
                findings: scoped_pair_findings(document, board_contours, target),
            })
        }
    }
}

/// Pair-linked gasket supports resolved on their author's contours and reflected here.
///
/// The author's own assembly is resolved against the author's accepted contours (its
/// configuration is read without the gasket projection, so this cannot recurse), and the
/// resolved placement, tangent and normal are mirrored through the pair map - the forward
/// map when the first member authors, its exact inverse when the second does. The
/// reflected placement is then associated with this half's own accepted outline: when the
/// destination contour cannot receive it, `fit_error` records that instead of snapping or
/// moving the geometry.
pub(crate) fn resolved_pair_gasket_supports(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    target: &MechanicalTarget,
    configuration: &MechanicalConfiguration,
) -> Result<Vec<MechanicalGasketSupport>, String> {
    let MechanicalTarget::Instance { instance_id } = target else {
        return Ok(Vec::new());
    };
    let Some(hardware) = document.hardware.as_ref() else {
        return Ok(Vec::new());
    };
    let Some(pair) = owning_pair(hardware, instance_id) else {
        return Ok(Vec::new());
    };
    if !pair
        .features
        .iter()
        .any(|feature| feature.kind == MechanicalFeatureKind::GasketSupport)
    {
        return Ok(Vec::new());
    }
    let Some(instance) = hardware
        .instances
        .iter()
        .find(|instance| instance.id == *instance_id)
    else {
        return Ok(Vec::new());
    };
    // A support sits on the case's expanded perimeter, not on the bare board outline,
    // so association allows for this half's wall and clearance.
    let destination = match crate::mechanical::expanded_outline(
        &destination_contours(board_contours, instance),
        configuration.clearance.max(0.0) + configuration.wall_thickness.max(0.0),
        document.revision,
    ) {
        Ok(expanded) => expanded,
        Err(_) => destination_contours(board_contours, instance),
    };
    let mut resolved = Vec::new();
    for feature in &pair.features {
        if feature.kind != MechanicalFeatureKind::GasketSupport {
            continue;
        }
        let Some(member) = feature
            .members
            .iter()
            .find(|member| member.instance_id == *instance_id)
        else {
            continue;
        };
        if member.state != MechanicalFeatureState::Linked {
            continue;
        }
        let Some(author_support) =
            author_resolved_gasket_support(document, board_contours, feature)?
        else {
            continue;
        };
        let mut support = if feature.author == *instance_id {
            author_support
        } else {
            let mapping = if pair
                .members
                .first()
                .is_some_and(|first| *first == feature.author)
            {
                pair.mapping
            } else {
                super::features::inverse_pose(pair.mapping)
            };
            super::features::mirror_support(mapping, &author_support)
        };
        support.pair_id = Some(feature.id.clone());
        if !destination.is_empty() && !super::inside_outline(support.at, &destination) {
            support.fit_error = Some(
                "The reflected gasket support does not associate with this half's board outline."
                    .into(),
            );
        }
        resolved.push(support);
    }
    Ok(resolved)
}

/// The author's resolved support, from the author's accepted contours.
fn author_resolved_gasket_support(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    feature: &MechanicalFeature,
) -> Result<Option<MechanicalGasketSupport>, String> {
    let author_target = MechanicalTarget::Instance {
        instance_id: feature.author.clone(),
    };
    // The author's configuration is read without the gasket projection so resolving its
    // own support cannot recurse through this path.
    let configuration =
        effective_inputs_inner(document, board_contours, &author_target, false)?.configuration;
    let Some(hardware) = document.hardware.as_ref() else {
        return Ok(None);
    };
    let Some(author) = hardware
        .instances
        .iter()
        .find(|instance| instance.id == feature.author)
    else {
        return Ok(None);
    };
    let mut author_document = document.clone();
    author_document.physical_instance_id = Some(feature.author.clone());
    author_document.mechanical = Some(configuration);
    // The author's own anchor is already projected into that configuration, so the
    // recursive resolve must not reflect any pair-linked gasket support itself: that
    // would re-enter this path and could not terminate for a two-way linked pair.
    if let Some(hardware) = author_document.hardware.as_mut() {
        for pair in hardware.pairs.iter_mut() {
            pair.features
                .retain(|feature| feature.kind != MechanicalFeatureKind::GasketSupport);
        }
    }
    if author.flipped {
        mirror_parts(&mut author_document.parts, document, &author.board_id);
    }
    let mut contours = declared_contours(board_contours, &author.board_id);
    if author.flipped {
        contours = mirror_contours(contours);
    }
    if contours.is_empty() {
        let (cache, _, _) = crate::geometry::outlines(document, None, &[]);
        contours = board_contours_for(document, &cache, &author.board_id);
    }
    let assembly = crate::mechanical::resolve(&author_document, &contours);
    let member_feature_id = feature
        .members
        .iter()
        .find(|member| member.instance_id == feature.author)
        .map(|member| member.feature_id.as_str());
    Ok(assembly
        .gasket_supports
        .into_iter()
        .find(|support| Some(support.id.as_str()) == member_feature_id))
}

/// Recomputes one board's contours when the caller has no accepted projection for it.
fn board_contours_for(
    document: &ProjectDoc,
    cache: &crate::geometry::OutlineCache,
    board_id: &str,
) -> Vec<Contour> {
    crate::geometry::board_contours(document, cache)
        .0
        .into_iter()
        .find(|entry| entry.board_id == board_id)
        .map(|entry| entry.contours)
        .unwrap_or_default()
}

/// The accepted contours of one board, or none when the projection lacks them.
fn declared_contours(board_contours: &[BoardContours], board_id: &str) -> Vec<Contour> {
    board_contours
        .iter()
        .find(|entry| entry.board_id == board_id)
        .map(|entry| entry.contours.clone())
        .unwrap_or_default()
}

/// The target board's effective parts, mirrored when the target half is flipped.
fn effective_parts(document: &ProjectDoc, board_id: &str, flipped: bool) -> Vec<Part> {
    let mut parts = document.parts.clone();
    if flipped {
        mirror_parts(&mut parts, document, board_id);
    }
    parts
}

/// Mirrors the selected board's parts into a flipped physical half's Case frame.
///
/// A part's `x` and rotation negate and its side swaps; its generator's `side`
/// parameter follows, so the resolver reads the same physical placement the accepted
/// scene carried. Parts owned by other boards are untouched.
pub(crate) fn mirror_parts(parts: &mut [Part], document: &ProjectDoc, board_id: &str) {
    let selected = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .map(|board| {
            board
                .part_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();
    for part in parts {
        if !selected.contains(&part.id) {
            continue;
        }
        // Ticket 12: mirror from the evaluated board pose, so half-frame
        // parts land board-space on the flipped half.
        part.pose = crate::matrix::layout::evaluated_part_pose(document, part);
        part.pose.at.x = -part.pose.at.x;
        part.pose.rotation = -part.pose.rotation;
        part.side = match part.side {
            Side::Front => Side::Back,
            Side::Back => Side::Front,
        };
        let parameters = part
            .generator_parameters
            .get_or_insert_with(Default::default);
        parameters.insert(
            "side".into(),
            serde_json::Value::String(if part.side == Side::Front { "F" } else { "B" }.into()),
        );
    }
}

/// The accepted mechanical pair findings that concern one board.
/// The accepted pair findings that concern one target.
///
/// A reading for one physical half sees a finding that names that half, or a finding
/// that names no half of the board at all (a board- or body-level problem). A finding
/// that names a *different* half of the same board is not this half's problem: two
/// halves sharing one PCB keep independent readiness, so a Case conflict on one half
/// cannot block the other's output.
fn scoped_pair_findings(
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
pub(crate) fn pair_findings(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
) -> Vec<Finding> {
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
                let outside = !contours.is_empty() && !super::inside_outline(point, &contours);
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

fn pair_finding(
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
fn destination_contours(
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
fn visible_linked_value(
    pair: &MechanicalMirrorPair,
    feature: &MechanicalFeature,
    instance_id: &str,
) -> Option<MechanicalFeatureValue> {
    if feature.author == instance_id {
        return Some(feature.value.clone());
    }
    super::features::counterpart(pair, feature, &feature.author)
        .ok()
        .flatten()
        .map(|derived| derived.value)
}

/// A linked feature's representative point in its destination frame.
///
/// `None` for a gasket support, whose placement only exists once its anchor has been
/// resolved on the author's contours.
fn value_point(value: &MechanicalFeatureValue) -> Option<Vec2> {
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
fn value_radius(value: &MechanicalFeatureValue) -> f64 {
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
fn local_conflict(
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

/// Mirrors accepted board contours into a flipped physical half's Case frame.
pub(crate) fn mirror_contours(mut contours: Vec<Contour>) -> Vec<Contour> {
    for contour in &mut contours {
        for point in &mut contour.points {
            point.x = -point.x;
        }
        contour.points.reverse();
    }
    contours
}

/// Projects a pair's linked features into one member's visible configuration.
///
/// A linked feature has exactly one authored value, stored on the pair in the
/// author's Case frame. Each member's visible geometry is derived from it here, so a
/// `Linked` feature is never also a second writable copy in the member's ordinary
/// configuration. A `Local` half keeps its own geometry (already in its ordinary
/// configuration) and an `Omitted` half shows none.
pub(crate) fn project_linked_features(
    hardware: &HardwareConfiguration,
    instance: &PhysicalBoardInstance,
    mut configuration: MechanicalConfiguration,
) -> Result<MechanicalConfiguration, String> {
    let Some(pair) = owning_pair(hardware, &instance.id) else {
        return Ok(configuration);
    };
    for feature in &pair.features {
        let member = feature
            .members
            .iter()
            .find(|member| member.instance_id == instance.id);
        let Some(member) = member else {
            continue;
        };
        match member.state {
            MechanicalFeatureState::Linked => {
                // The authored half shows the stored value; the other half shows the
                // derived counterpart. Gasket support anchors keep their
                // author-contour fraction: reflecting the resolved placement is the
                // assembly resolver's job, not a reinterpretation here.
                let value = if feature.author == instance.id {
                    feature.value.clone()
                } else {
                    match super::features::counterpart(pair, feature, &feature.author)? {
                        Some(derived) => derived.value,
                        None => continue,
                    }
                };
                super::edit::install_feature(
                    &mut configuration,
                    feature.kind,
                    &member.feature_id,
                    &value,
                )?;
            }
            MechanicalFeatureState::Local => {}
            MechanicalFeatureState::Omitted => {
                super::edit::remove_feature(&mut configuration, feature.kind, &member.feature_id);
            }
        }
    }
    Ok(configuration)
}

fn board_thickness(document: &ProjectDoc, board_id: &str) -> Result<f64, String> {
    document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .map(|board| board.thickness)
        .ok_or_else(|| format!("physical instance board {board_id:?} is not in the document"))
}

/// The shared construction source: the explicit shared construction, or the
/// first instance that carries a configuration.
fn shared_source(hardware: &HardwareConfiguration) -> Option<&MechanicalConfiguration> {
    hardware.shared_construction.as_ref().or_else(|| {
        hardware
            .instances
            .iter()
            .find_map(|entry| entry.mechanical.as_ref())
    })
}

fn resolve_instance_configuration(
    hardware: &HardwareConfiguration,
    instance: &PhysicalBoardInstance,
    board_thickness: f64,
) -> Result<MechanicalConfiguration, String> {
    match instance.construction_ownership {
        MechanicalConstructionOwnership::Legacy => {
            legacy_configuration(hardware, instance, board_thickness)
        }
        MechanicalConstructionOwnership::Local => local_configuration(instance, board_thickness),
        MechanicalConstructionOwnership::Pair => match owning_pair(hardware, &instance.id) {
            Some(pair) => pair_configuration(instance, pair, board_thickness),
            None => local_configuration(instance, board_thickness),
        },
    }
}

fn local_configuration(
    instance: &PhysicalBoardInstance,
    board_thickness: f64,
) -> Result<MechanicalConfiguration, String> {
    let mut configuration = instance.mechanical.clone().ok_or_else(|| {
        format!(
            "physical instance {:?} has no local mechanical configuration",
            instance.id
        )
    })?;
    configuration.board_id = instance.board_id.clone();
    configuration.pcb_thickness = board_thickness;
    Ok(configuration)
}

fn legacy_configuration(
    hardware: &HardwareConfiguration,
    instance: &PhysicalBoardInstance,
    board_thickness: f64,
) -> Result<MechanicalConfiguration, String> {
    let common = shared_source(hardware);
    let mut configuration = instance
        .mechanical
        .clone()
        .or_else(|| common.cloned())
        .ok_or_else(|| {
            format!(
                "physical instance {:?} has no mechanical configuration or shared construction",
                instance.id
            )
        })?;
    if instance.mechanical.is_none() {
        configuration.openings = None;
        configuration.mounts.clear();
        configuration.closure_mounts = None;
        configuration.battery = None;
        configuration.battery_height = 0.0;
    }
    if let Some(common) = common {
        configuration = overlay_shared(configuration, common, instance_supports(instance))?;
    }
    configuration.board_id = instance.board_id.clone();
    configuration.pcb_thickness = board_thickness;
    Ok(configuration)
}

fn pair_configuration(
    instance: &PhysicalBoardInstance,
    pair: &MechanicalMirrorPair,
    board_thickness: f64,
) -> Result<MechanicalConfiguration, String> {
    let configuration = instance.mechanical.clone().ok_or_else(|| {
        format!(
            "physical instance {:?} has no local mechanical configuration",
            instance.id
        )
    })?;
    let mut configuration = overlay_shared(
        configuration,
        &pair.construction,
        instance_supports(instance),
    )?;
    configuration.board_id = instance.board_id.clone();
    configuration.pcb_thickness = board_thickness;
    Ok(configuration)
}

fn owning_pair<'a>(
    hardware: &'a HardwareConfiguration,
    instance_id: &str,
) -> Option<&'a MechanicalMirrorPair> {
    hardware
        .pairs
        .iter()
        .find(|pair| pair.members.iter().any(|member| member == instance_id))
}

fn instance_supports(instance: &PhysicalBoardInstance) -> Vec<MechanicalGasketAnchor> {
    instance
        .mechanical
        .as_ref()
        .and_then(|configuration| configuration.gasket_layout.as_ref())
        .map(|layout| layout.supports.clone())
        .unwrap_or_default()
}

/// Overlays a shared construction's fields and its gasket layout (keeping the
/// instance's own supports) onto an instance configuration.
fn overlay_shared<S: serde::Serialize>(
    configuration: MechanicalConfiguration,
    shared: &S,
    supports: Vec<MechanicalGasketAnchor>,
) -> Result<MechanicalConfiguration, String> {
    let mut output = serde_json::to_value(configuration)
        .map_err(|error| error.to_string())?
        .as_object()
        .cloned()
        .ok_or_else(|| "mechanical configuration did not serialize to an object".to_string())?;
    let shared = serde_json::to_value(shared).map_err(|error| error.to_string())?;
    let shared = shared
        .as_object()
        .ok_or_else(|| "shared construction did not serialize to an object".to_string())?;
    for field in SHARED_OVERLAY_FIELDS {
        if let Some(value) = shared.get(field) {
            output.insert(field.into(), value.clone());
        }
    }
    if let Some(mut gasket_layout) = shared.get("gasketLayout").cloned() {
        if let Some(layout) = gasket_layout.as_object_mut() {
            layout.insert(
                "supports".into(),
                serde_json::to_value(supports).map_err(|error| error.to_string())?,
            );
        }
        output.insert("gasketLayout".into(), gasket_layout);
    }
    serde_json::from_value(serde_json::Value::Object(output)).map_err(|error| error.to_string())
}

/// Applies the defaults every effective configuration needs on top of its stored value.
///
/// Ports the adapter's remaining `mechanical_defaults` computation so `effective_inputs`
/// is the single source of the fully defaulted configuration:
///
/// - the wireless battery default sits at the board-local part center, mirrored across
///   X for a flipped instance, and drives `battery_height`;
/// - the stabilizer defaults propose a footprint for every wide MX switch keycap that
///   has no existing `STAB_MX_` footprint, rounding the key units and deriving the
///   rotation. Explicit stabilizer overrides keep every field except the recomputed
///   units and rotation.
///
/// The host `cad_jobs::mechanical_defaults` is now a thin delegate to this function.
pub fn apply_mechanical_defaults(
    document: &ProjectDoc,
    mut configuration: MechanicalConfiguration,
    board_id: &str,
    flipped: bool,
) -> MechanicalConfiguration {
    let board = document.boards.iter().find(|board| board.id == board_id);
    let parts = document
        .parts
        .iter()
        .filter(|part| board.is_some_and(|board| board.part_ids.contains(&part.id)))
        .collect::<Vec<_>>();
    fn center(parts: &[&Part], coordinate: impl Fn(&Part) -> f64) -> f64 {
        let values = parts
            .iter()
            .map(|part| coordinate(part))
            .collect::<Vec<_>>();
        values
            .iter()
            .copied()
            .reduce(f64::min)
            .zip(values.iter().copied().reduce(f64::max))
            .map_or(0.0, |(min, max)| (min + max) / 2.0)
    }
    let at = Vec2 {
        x: center(&parts, |part| part.pose.at.x) * if flipped { -1.0 } else { 1.0 },
        y: center(&parts, |part| part.pose.at.y),
    };
    if configuration.battery.is_none()
        && document
            .hardware
            .as_ref()
            .is_some_and(|hardware| hardware.transport == HardwareTransport::Wireless)
    {
        configuration.battery = Some(MechanicalBattery {
            cable_width: Some(2.0),
            size: Vec3 {
                x: 30.0,
                y: 20.0,
                z: 6.0,
            },
            at,
            cable_exit: Vec2 {
                x: at.x + 17.0,
                y: at.y,
            },
        });
    }
    if let Some(battery) = &configuration.battery {
        configuration.battery_height = battery.size.z;
    }

    let explicit = configuration.stabilizers.clone().unwrap_or_default();
    let mut stabilizers = Vec::new();
    for part in &parts {
        let Some(definition) = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
        else {
            continue;
        };
        let size = part.keycap.as_ref().or(definition.keycap.as_ref());
        let Some(size) = size.filter(|size| size.x.max(size.y) >= 37.0) else {
            continue;
        };
        if definition
            .generator
            .as_ref()
            .is_none_or(|generator| generator.source.to_lowercase() != "ceoloide/switch_mx")
        {
            continue;
        }
        let has_existing = document.parts.iter().any(|candidate| {
            let is_stabilizer = document
                .definitions
                .iter()
                .find(|entry| entry.id == candidate.definition_id)
                .and_then(|entry| entry.kicad_source.as_ref())
                .is_some_and(|source| source.source.contains("(footprint \"STAB_MX_"));
            is_stabilizer
                && (candidate.pose.at.x - part.pose.at.x)
                    .hypot(candidate.pose.at.y - part.pose.at.y)
                    < 0.01
        });
        if has_existing {
            continue;
        }
        let key_units = (((size.x.max(size.y) + 1.0) / 19.05) * 4.0).round() / 4.0;
        let units = if key_units <= 2.75 { 2.0 } else { key_units };
        let rotation = if size.y > size.x { 90.0 } else { 0.0 };
        let mut stabilizer = explicit
            .iter()
            .find(|entry| entry.part_id == part.id)
            .cloned()
            .unwrap_or(MechanicalStabilizerOverride {
                profile: None,
                part_id: part.id.clone(),
                kind: MechanicalStabilizerKind::PcbMount,
                units,
                rotation: Some(rotation),
            });
        stabilizer.units = units;
        stabilizer.rotation = Some(rotation);
        stabilizers.push(stabilizer);
    }
    configuration.stabilizers = Some(stabilizers);
    configuration
}

/// Projects the effective document a target's mechanical resolution sees.
///
/// Installs the resolved effective configuration as `document.mechanical` and, for a
/// physical instance, records `document.physical_instance_id`. The remaining
/// defaulting (battery placement and stabilizer proposals) lives in
/// [`apply_mechanical_defaults`], which [`effective_inputs`] already applies, so the
/// projected document carries the fully defaulted configuration.
///
/// A flipped instance is mirrored exactly as the host `effective_case_inputs` does:
/// the target board's parts get `pose.at.x = -x`, `pose.rotation = -rotation`, a
/// swapped `side` and a rewritten `side` generator parameter; that board's
/// `board_references` are disabled; and the board's contours are mirrored by negating
/// each point's X and reversing the point order. The mirrored contours are returned
/// so callers can resolve the assembly and replace the scene's target-board contours
/// without recomputing them.
///
/// A missing canonical or instance configuration is not an error here: the structural
/// projection (instance identity, part mirroring, contours) is still meaningful, and
/// callers that require a configuration use [`effective_inputs`] directly. This matches
/// the host projection's previous leniency.
pub fn effective_document(
    document: &ProjectDoc,
    target: &MechanicalTarget,
) -> Result<(ProjectDoc, Vec<Contour>), String> {
    // `geometry::board_contours` reads the outline path cache, so build it from the
    // source document first; a default (empty) cache would resolve no contours. The
    // accepted contours feed the effective read and are mirrored by it for a flipped
    // instance, so the same geometry the scene carried is used here.
    let (cache, _, _) = crate::geometry::outlines(document, None, &[]);
    let (per_board, _, _) = board_contours(document, &cache);
    let mut effective = document.clone();
    let (board_id, flipped, contours) = match target {
        MechanicalTarget::Board { board_id } => {
            // A canonical configuration that belongs to another board is still
            // projected with this board's defaults, matching the host's previous
            // unconditional `document.mechanical.clone()` behaviour.
            let inputs = effective_inputs(document, &per_board, target).ok();
            effective.mechanical = document.mechanical.clone().map(|configuration| {
                inputs
                    .as_ref()
                    .map(|inputs| inputs.configuration.clone())
                    .unwrap_or_else(|| {
                        apply_mechanical_defaults(document, configuration, board_id, false)
                    })
            });
            let contours = inputs
                .map(|inputs| inputs.board_contours)
                .unwrap_or_else(|| declared_contours(&per_board, board_id));
            (board_id.clone(), false, contours)
        }
        MechanicalTarget::Instance { instance_id } => {
            let instance = document
                .hardware
                .as_ref()
                .and_then(|hardware| {
                    hardware
                        .instances
                        .iter()
                        .find(|instance| instance.id == *instance_id)
                })
                .ok_or("The selected physical instance is unavailable")?;
            effective.physical_instance_id = Some(instance_id.clone());
            let inputs = effective_inputs(document, &per_board, target).ok();
            let contours = match &inputs {
                Some(inputs) => inputs.board_contours.clone(),
                None => {
                    let declared = declared_contours(&per_board, &instance.board_id);
                    if instance.flipped {
                        mirror_contours(declared)
                    } else {
                        declared
                    }
                }
            };
            effective.mechanical = inputs.map(|inputs| inputs.configuration);
            (instance.board_id.clone(), instance.flipped, contours)
        }
    };

    if flipped {
        mirror_parts(&mut effective.parts, document, &board_id);
        for reference in &mut effective.board_references {
            if reference.board_id == board_id {
                reference.enabled = false;
            }
        }
    }

    Ok((effective, contours))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Board, HardwareConfiguration, HardwareTopology, HardwareTransport,
        MechanicalConstructionOwnership, MechanicalGasketAnchor, MechanicalGasketLayout,
        MechanicalMirrorPair, Part, PhysicalBoardInstance, Pose2, Side, Vec2,
    };

    /// Test adapter: these configuration cases are independent of the accepted
    /// contours, so they resolve without a contour projection.
    fn resolve_inputs(
        document: &ProjectDoc,
        target: &MechanicalTarget,
    ) -> Result<MechanicalInputs, String> {
        effective_inputs(document, &[], target)
    }

    fn board(id: &str, thickness: f64) -> Board {
        Board {
            id: id.into(),
            name: id.into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness,
            traces: vec![],
            vias: vec![],
        }
    }

    fn configuration(board_id: &str) -> MechanicalConfiguration {
        serde_json::from_value(serde_json::json!({
            "boardId": board_id,
            "method": "printed",
            "mount": "tray",
            "plateThickness": 1.5,
            "plateFoamThickness": 1.0,
            "pcbThickness": 1.6,
            "bottomFoamThickness": 0.5,
            "batteryHeight": 0.0,
            "bottomThickness": 2.0,
            "plateToPcb": 3.5,
            "wallThickness": 2.0,
            "clearance": 0.2,
            "profiles": []
        }))
        .unwrap()
    }

    fn part(id: &str, x: f64) -> Part {
        Part {
            id: id.into(),
            definition_id: "switch".into(),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2 { x, y: 2.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn instance(
        id: &str,
        board_id: &str,
        ownership: MechanicalConstructionOwnership,
        mechanical: Option<MechanicalConfiguration>,
    ) -> PhysicalBoardInstance {
        PhysicalBoardInstance {
            id: id.into(),
            name: id.into(),
            board_id: board_id.into(),
            half: "left".into(),
            role: "controller".into(),
            flipped: false,
            controller_part_id: None,
            mechanical,
            construction_ownership: ownership,
            mechanical_identity_reservation: None,
        }
    }

    fn hardware(
        instances: Vec<PhysicalBoardInstance>,
        shared: Option<MechanicalConfiguration>,
        transport: HardwareTransport,
    ) -> HardwareConfiguration {
        HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport,
            instances,
            boards: vec![],
            shared_construction: shared,
            pairs: vec![],
        }
    }

    fn document_with(board: Board, hardware: HardwareConfiguration) -> ProjectDoc {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board);
        document.hardware = Some(hardware);
        document
    }

    fn mount(id: &str) -> Mount {
        serde_json::from_value(serde_json::json!({
            "id": id, "at": { "x": 1.0, "y": 2.0 }, "kind": "hole", "holeDiameter": 2.2
        }))
        .unwrap()
    }

    fn opening(id: &str) -> MechanicalOpening {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "vertices": [{ "id": "vertex", "point": { "x": 0.0, "y": 0.0 } }],
            "z": 0.0,
            "height": 1.0
        }))
        .unwrap()
    }

    fn battery() -> MechanicalBattery {
        serde_json::from_value(serde_json::json!({
            "size": { "x": 30.0, "y": 20.0, "z": 6.0 },
            "at": { "x": 1.0, "y": 2.0 },
            "cableExit": { "x": 3.0, "y": 2.0 }
        }))
        .unwrap()
    }

    fn stabilizer() -> MechanicalStabilizerOverride {
        serde_json::from_value(serde_json::json!({
            "partId": "part", "kind": "pcb-mount", "units": 2.0
        }))
        .unwrap()
    }

    fn profile(id: &str) -> MechanicalPartProfile {
        serde_json::from_value(serde_json::json!({
            "definitionId": id, "source": "test", "cutouts": [], "plateToPcb": 3.5
        }))
        .unwrap()
    }

    fn hardware_specification(id: &str) -> MechanicalHardwareSpecification {
        serde_json::from_value(serde_json::json!({
            "id": id, "partId": "part", "featureId": "feature", "designation": "M2",
            "thread": "M2", "length": 8.0, "quantity": 1
        }))
        .unwrap()
    }

    fn critical_fit(id: &str) -> MechanicalCriticalFit {
        serde_json::from_value(serde_json::json!({
            "id": id, "partId": "part", "label": "fit",
            "from": { "x": 0.0, "y": 0.0 }, "to": { "x": 1.0, "y": 1.0 },
            "tolerance": "0.1"
        }))
        .unwrap()
    }

    fn anchor(id: &str) -> MechanicalGasketAnchor {
        serde_json::from_value(serde_json::json!({
            "id": id, "regionId": "region", "outlineKey": "outline", "anchor": 0.0
        }))
        .unwrap()
    }

    fn mirror_pair(
        id: &str,
        members: [&str; 2],
        construction: serde_json::Value,
    ) -> MechanicalMirrorPair {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "members": members,
            "mapping": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 },
            "construction": construction
        }))
        .unwrap()
    }

    fn gasket_layout(id: &str, supports: Vec<MechanicalGasketAnchor>) -> MechanicalGasketLayout {
        let _ = id;
        serde_json::from_value(serde_json::json!({
            "length": 99.0,
            "width": 3.0,
            "thickness": 2.0,
            "compression": 0.15,
            "supports": supports
        }))
        .unwrap()
    }

    #[test]
    fn canonical_configuration_resolves_to_itself() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board("board", 1.6));
        let mut canonical = configuration("board");
        canonical.clearance = 0.4;
        // The defaulting pass always materializes the stabilizer proposal list.
        canonical.stabilizers = Some(vec![]);
        document.mechanical = Some(canonical.clone());

        let inputs = resolve_inputs(
            &document,
            &MechanicalTarget::Board {
                board_id: "board".into(),
            },
        )
        .unwrap();

        assert_eq!(inputs.board_id, "board");
        assert_eq!(inputs.configuration, canonical);
    }

    #[test]
    fn legacy_instance_overlays_the_shared_construction() {
        let mut common = configuration("shared");
        common.method = PlateMethod::Cnc;
        common.mount = MechanicalMount::Rigid;
        common.plate_thickness = 4.0;
        common.profiles = vec![profile("shared")];
        common.hardware = Some(vec![hardware_specification("shared")]);
        common.critical_fits = Some(vec![critical_fit("shared")]);
        common.gasket_layout = Some(gasket_layout("shared", vec![anchor("shared-support")]));

        let mut local = configuration("board");
        local.method = PlateMethod::Printed;
        local.plate_thickness = 1.0;
        local.openings = Some(vec![opening("local-opening")]);
        local.mounts = vec![mount("local-mount")];
        local.closure_mounts = Some(vec![mount("local-closure")]);
        local.battery = Some(battery());
        local.stabilizers = Some(vec![stabilizer()]);
        local.gasket_layout = Some(gasket_layout("local", vec![anchor("local-support")]));

        let document = document_with(
            board("board", 1.6),
            hardware(
                vec![instance(
                    "left",
                    "board",
                    MechanicalConstructionOwnership::Legacy,
                    Some(local),
                )],
                Some(common.clone()),
                HardwareTransport::None,
            ),
        );

        let inputs = resolve_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .unwrap();
        let configuration = inputs.configuration;

        // Overlaid shared fields.
        assert_eq!(configuration.method, PlateMethod::Cnc);
        assert_eq!(configuration.mount, MechanicalMount::Rigid);
        assert_eq!(configuration.plate_thickness, 4.0);
        assert_eq!(configuration.profiles, common.profiles);
        assert_eq!(configuration.hardware, common.hardware);
        assert_eq!(configuration.critical_fits, common.critical_fits);
        // The shared layout keeps the instance's own supports.
        let layout = configuration.gasket_layout.clone().unwrap();
        assert_eq!(layout.length, 99.0);
        assert_eq!(layout.supports, vec![anchor("local-support")]);
        // Local-only fields survive the overlay.
        assert_eq!(configuration.openings, Some(vec![opening("local-opening")]));
        assert_eq!(configuration.mounts, vec![mount("local-mount")]);
        assert_eq!(
            configuration.closure_mounts,
            Some(vec![mount("local-closure")])
        );
        assert_eq!(configuration.battery, Some(battery()));
        // The defaulting pass recomputes the stabilizer proposal list; this board
        // has no wide MX keycap part, so the list is empty.
        assert_eq!(configuration.stabilizers, Some(vec![]));
        assert_eq!(configuration.board_id, "board");
        assert_eq!(configuration.pcb_thickness, 1.6);
    }

    #[test]
    fn legacy_instance_without_a_shared_source_falls_back_to_the_first_instance() {
        let mut primary = configuration("board");
        primary.method = PlateMethod::Cnc;
        primary.plate_thickness = 3.0;
        primary.openings = Some(vec![opening("primary-opening")]);
        primary.mounts = vec![mount("primary-mount")];

        let document = document_with(
            board("board", 1.6),
            hardware(
                vec![
                    instance(
                        "primary",
                        "board",
                        MechanicalConstructionOwnership::Legacy,
                        Some(primary),
                    ),
                    instance(
                        "secondary",
                        "board",
                        MechanicalConstructionOwnership::Legacy,
                        None,
                    ),
                ],
                None,
                HardwareTransport::None,
            ),
        );

        let inputs = resolve_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "secondary".into(),
            },
        )
        .unwrap();
        let configuration = inputs.configuration;

        assert_eq!(configuration.method, PlateMethod::Cnc);
        assert_eq!(configuration.plate_thickness, 3.0);
        // The target had no local configuration, so its board-local features clear.
        assert!(configuration.openings.is_none());
        assert!(configuration.mounts.is_empty());
        assert!(configuration.closure_mounts.is_none());
        assert!(configuration.battery.is_none());
        assert_eq!(configuration.board_id, "board");
        assert_eq!(configuration.pcb_thickness, 1.6);
    }

    #[test]
    fn local_instance_ignores_the_shared_overlay() {
        let mut common = configuration("shared");
        common.method = PlateMethod::Cnc;
        common.plate_thickness = 9.0;

        let mut local = configuration("board");
        local.method = PlateMethod::Printed;
        local.plate_thickness = 1.0;

        let document = document_with(
            board("board", 1.6),
            hardware(
                vec![instance(
                    "left",
                    "board",
                    MechanicalConstructionOwnership::Local,
                    Some(local),
                )],
                Some(common),
                HardwareTransport::None,
            ),
        );

        let inputs = resolve_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .unwrap();

        assert_eq!(inputs.configuration.method, PlateMethod::Printed);
        assert_eq!(inputs.configuration.plate_thickness, 1.0);
    }

    #[test]
    fn pair_instance_uses_the_pairs_shared_construction() {
        let mut local = configuration("board");
        local.method = PlateMethod::Printed;
        local.plate_thickness = 1.0;
        local.openings = Some(vec![opening("local-opening")]);

        let mut hardware_with_pair = hardware(
            vec![instance(
                "left",
                "board",
                MechanicalConstructionOwnership::Pair,
                Some(local),
            )],
            None,
            HardwareTransport::None,
        );
        hardware_with_pair.pairs.push(mirror_pair(
            "pair-left-right",
            ["left", "right"],
            serde_json::json!({
                "method": "cnc", "mount": "rigid", "integratedPlateFrame": true,
                "plateThickness": 7.0, "plateFoamThickness": 0.5,
                "bottomFoamThickness": 0.25, "bottomThickness": 3.0,
                "plateToPcb": 4.0, "wallThickness": 2.5, "clearance": 0.3
            }),
        ));
        let document = document_with(board("board", 1.6), hardware_with_pair);

        let inputs = resolve_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .unwrap();

        assert_eq!(inputs.configuration.method, PlateMethod::Cnc);
        assert_eq!(inputs.configuration.plate_thickness, 7.0);
        assert_eq!(
            inputs.configuration.openings,
            Some(vec![opening("local-opening")])
        );
        assert_eq!(inputs.configuration.board_id, "board");
        assert_eq!(inputs.configuration.pcb_thickness, 1.6);

        // A `Pair` instance with no owning pair behaves local.
        let mut hardware_with_pair = hardware(
            vec![instance(
                "solo",
                "board",
                MechanicalConstructionOwnership::Pair,
                Some(configuration("board")),
            )],
            None,
            HardwareTransport::None,
        );
        hardware_with_pair.pairs.push(mirror_pair(
            "pair-a-b",
            ["a", "b"],
            serde_json::json!({
                "method": "cnc", "mount": "rigid", "integratedPlateFrame": true,
                "plateThickness": 7.0, "plateFoamThickness": 0.5,
                "bottomFoamThickness": 0.25, "bottomThickness": 3.0,
                "plateToPcb": 4.0, "wallThickness": 2.5, "clearance": 0.3
            }),
        ));
        let document = document_with(board("board", 1.6), hardware_with_pair);
        let inputs = resolve_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "solo".into(),
            },
        )
        .unwrap();
        assert_eq!(inputs.configuration.method, PlateMethod::Printed);
    }

    #[test]
    fn flipped_instance_keeps_its_board_local_reflection() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        let mut board = board("board", 1.6);
        board.part_ids = vec!["part-a".into(), "part-b".into()];
        document.boards.push(board);
        document.parts = vec![part("part-a", 3.0), part("part-b", 8.0)];
        let mut flipped = instance(
            "left",
            "board",
            MechanicalConstructionOwnership::Legacy,
            Some(configuration("board")),
        );
        flipped.flipped = true;
        document.hardware = Some(hardware(vec![flipped], None, HardwareTransport::Wireless));

        let inputs = resolve_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .unwrap();
        let battery = inputs.configuration.battery.expect("wireless default");

        // Board-local part center x is (3 + 8) / 2 = 5.5; the flip reflects it.
        assert!((battery.at.x + 5.5).abs() < 1e-9, "{:?}", battery.at);
        assert!((battery.at.y - 2.0).abs() < 1e-9, "{:?}", battery.at);
        assert!((battery.cable_exit.x - (-5.5 + 17.0)).abs() < 1e-9);
        assert_eq!(inputs.configuration.battery_height, 6.0);
    }

    #[test]
    fn wireless_transport_supplies_the_battery_default_when_none_is_stored() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        let mut board = board("board", 1.6);
        board.part_ids = vec!["part-a".into(), "part-b".into()];
        document.boards.push(board);
        document.parts = vec![part("part-a", 3.0), part("part-b", 8.0)];
        document.mechanical = Some(configuration("board"));
        document.hardware = Some(hardware(vec![], None, HardwareTransport::Wireless));

        let inputs = resolve_inputs(
            &document,
            &MechanicalTarget::Board {
                board_id: "board".into(),
            },
        )
        .unwrap();
        let default_battery = inputs.configuration.battery.expect("wireless default");

        assert!((default_battery.at.x - 5.5).abs() < 1e-9);
        assert_eq!(default_battery.size.z, 6.0);
        assert_eq!(inputs.configuration.battery_height, 6.0);

        // A stored battery is never replaced by the default.
        document.mechanical.as_mut().unwrap().battery = Some(battery());
        let inputs = resolve_inputs(
            &document,
            &MechanicalTarget::Board {
                board_id: "board".into(),
            },
        )
        .unwrap();
        assert_eq!(inputs.configuration.battery, Some(battery()));
    }

    #[test]
    fn ineligible_targets_return_an_error() {
        // No canonical configuration at all.
        let empty = ProjectDoc::empty("doc", "Fixture");
        assert!(
            resolve_inputs(
                &empty,
                &MechanicalTarget::Board {
                    board_id: "board".into(),
                },
            )
            .is_err()
        );

        // An eligible board resolves the Core-owned defaults when it has no stored
        // canonical configuration (so the adapter can drop its own defaults builder).
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board("board", 1.6));
        let defaults = resolve_inputs(
            &document,
            &MechanicalTarget::Board {
                board_id: "board".into(),
            },
        )
        .unwrap();
        assert_eq!(defaults.board_id, "board");
        assert_eq!(defaults.configuration.board_id, "board");
        assert!(
            defaults.configuration.mounts.is_empty(),
            "a defaulted board keeps no authored mounts"
        );

        // A canonical configuration that belongs to another board is not borrowed:
        // the requested board still resolves its own defaults.
        document.mechanical = Some(configuration("other"));
        let other = resolve_inputs(
            &document,
            &MechanicalTarget::Board {
                board_id: "board".into(),
            },
        )
        .unwrap();
        assert_eq!(other.configuration.board_id, "board");

        // No hardware configuration at all.
        assert!(
            resolve_inputs(
                &document,
                &MechanicalTarget::Instance {
                    instance_id: "left".into(),
                },
            )
            .is_err()
        );

        // Hardware without the requested instance.
        document.hardware = Some(hardware(vec![], None, HardwareTransport::None));
        assert!(
            resolve_inputs(
                &document,
                &MechanicalTarget::Instance {
                    instance_id: "left".into(),
                },
            )
            .is_err()
        );

        // Instance whose board is not in the document.
        document.hardware = Some(hardware(
            vec![instance(
                "left",
                "ghost",
                MechanicalConstructionOwnership::Legacy,
                Some(configuration("ghost")),
            )],
            None,
            HardwareTransport::None,
        ));
        assert!(
            resolve_inputs(
                &document,
                &MechanicalTarget::Instance {
                    instance_id: "left".into(),
                },
            )
            .is_err()
        );
    }

    #[test]
    fn a_flipped_instance_mirrors_parts_references_and_contours() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.outline.push(OutlineFeature::Rect {
            anchor_part_id: None,
            id: "outline".into(),
            center: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 80.0, y: 60.0 },
            radius: 0.0,
            rotation: None,
            operation: Operation::Add,
        });
        let mut board = board("board", 1.6);
        board.outline_ids = vec!["outline".into()];
        board.part_ids = vec!["left-switch".into(), "right-switch".into()];
        document.boards.push(board);
        document.parts.push(part("left-switch", 3.0));
        document.parts.push(part("right-switch", 40.0));
        document.board_references.push(
            serde_json::from_value(serde_json::json!({
                "id": "reference", "boardId": "board", "assetId": "asset",
                "enabled": true,
                "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 },
                "elevation": 0.0, "modelAssets": {}
            }))
            .unwrap(),
        );
        let mut flipped = instance(
            "left",
            "board",
            MechanicalConstructionOwnership::Local,
            Some(configuration("board")),
        );
        flipped.flipped = true;
        document.hardware = Some(hardware(vec![flipped], None, HardwareTransport::None));

        let (effective, contours) = effective_document(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .unwrap();

        assert_eq!(effective.physical_instance_id.as_deref(), Some("left"));
        let mirrored = effective
            .parts
            .iter()
            .find(|part| part.id == "left-switch")
            .unwrap();
        assert_eq!(mirrored.pose.at.x, -3.0);
        assert_eq!(mirrored.side, Side::Back);
        assert_eq!(
            mirrored.generator_parameters.as_ref().unwrap()["side"],
            serde_json::Value::String("B".into())
        );
        assert_eq!(
            effective
                .parts
                .iter()
                .find(|part| part.id == "right-switch")
                .unwrap()
                .pose
                .at
                .x,
            -40.0
        );
        assert!(!effective.board_references[0].enabled);

        // The returned target contours negate X and reverse the point order.
        let (cache, _, _) = crate::geometry::outlines(&document, None, &[]);
        let (per_board, _, _) = crate::geometry::board_contours(&document, &cache);
        let mut expected = per_board
            .into_iter()
            .find(|entry| entry.board_id == "board")
            .unwrap()
            .contours;
        for contour in &mut expected {
            for point in &mut contour.points {
                point.x = -point.x;
            }
            contour.points.reverse();
        }
        assert_eq!(contours, expected);
        assert!(!contours.is_empty(), "the board outline resolves");
    }
}
