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

mod pair_findings;
pub use pair_findings::pair_findings;
use pair_findings::{destination_contours, scoped_pair_findings};

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
pub fn resolved_pair_gasket_supports(
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
pub fn mirror_parts(parts: &mut [Part], document: &ProjectDoc, board_id: &str) {
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

/// Mirrors accepted board contours into a flipped physical half's Case frame.
pub fn mirror_contours(mut contours: Vec<Contour>) -> Vec<Contour> {
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
pub fn project_linked_features(
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
mod tests;
