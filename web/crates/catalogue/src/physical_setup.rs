//! Private physical-setup proposal rules. This code prepares values only; it never submits edits.

#[cfg(test)]
use boardstudio_core::CoreEngine;
use boardstudio_core::mechanical::MechanicalTarget;
use boardstudio_core::mechanical::change::MechanicalFeatureCorrespondence;
use boardstudio_core::mechanical::inputs::effective_inputs;
#[cfg(test)]
use boardstudio_core::mechanical::pair;
#[cfg(test)]
use boardstudio_core::model::{CoreReply, CoreRequest, EditCommand, EditPhase};
use boardstudio_core::model::{
    EditOperation, HardwareTopology, HardwareTransport, MechanicalConfiguration,
    MechanicalConstructionOwnership, MechanicalFeatureKind, MirroredHalfLabel, MirroredHalfLabels,
    MirroredHalfMode, MirroredHalfPhysicalSetup, MirroredIdRule, PartDefinition,
    PhysicalBoardInstance, Pose2, ProjectDoc, Vec2,
};
use serde_json::Value;
use std::collections::BTreeSet;

/// Presentation-owned event data; generated IDs are captured before proposal work starts.
#[derive(Clone, Debug, PartialEq)]
pub enum SetupIntent {
    Topology {
        board_id: String,
        selected_instance_id: Option<String>,
        split: bool,
        new_primary_id: String,
        new_secondary_id: String,
    },
    Transport(HardwareTransport),
    ReversibleLayout(bool),
    /// Establishes an explicit mechanical mirror pair between two physical halves
    /// that already exist, such as independently created or distinct-PCB halves.
    ///
    /// The reviewed choices travel with the intent. `mapping` is required whenever
    /// the halves' declared transforms do not imply a representable map (translated
    /// or rotated distinct board frames); `None` accepts only the physical
    /// reflection opposite flips already imply. `None` for `construction_source`
    /// takes the first member, and `None` for `correspondences` matches features
    /// that share a durable id. Nothing is inferred from coordinates, names or
    /// `half` labels.
    LinkMirrorPair {
        pair_id: String,
        first_instance_id: String,
        second_instance_id: String,
        mapping: Option<Pose2>,
        construction_source: Option<String>,
        correspondences: Option<Vec<MechanicalFeatureCorrespondence>>,
    },
}

/// The reviewed `LinkMirrorPair` payload for a setup-established mechanical pair.
///
/// This is the pair data the proposal returns so the Case adapter can submit one typed edit
/// and the Session settles the complete pair with one Undo step. The persisted pair
/// (construction subset, features and geometry) is derived by Core at apply time;
/// this payload never duplicates those rules.
#[derive(Clone, Debug, PartialEq)]
pub struct MirrorPairLink {
    pub pair_id: String,
    pub members: [String; 2],
    pub mapping: Pose2,
    pub construction_source: String,
    pub correspondences: Vec<MechanicalFeatureCorrespondence>,
}

/// A physical-setup proposal: the document the setup would accept plus the explicit
/// mechanical mirror pair it establishes.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalSetupProposal {
    /// The proposed document. It carries any new halves but never the pair itself:
    /// the pair travels as [`MirrorPairLink`] so it is not applied twice.
    pub document: ProjectDoc,
    /// The pair this setup establishes, if the targets are eligible.
    pub mirror_pair: Option<MirrorPairLink>,
}

impl PhysicalSetupProposal {
    /// Turns an eligible topology setup into the same typed Core operation used by Layout.
    pub fn into_mirrored_operation(
        &self,
        mounting_hole: Option<&PartDefinition>,
    ) -> Result<Option<(Vec<String>, EditOperation)>, String> {
        let Some(link) = self.mirror_pair.as_ref() else {
            return Ok(None);
        };
        let hardware = self
            .document
            .hardware
            .as_ref()
            .ok_or("Physical instances are unavailable")?;
        let source = hardware
            .instances
            .iter()
            .find(|item| item.id == link.members[0])
            .ok_or("Physical setup source instance is unavailable")?
            .clone();
        let mirror = hardware
            .instances
            .iter()
            .find(|item| item.id == link.members[1])
            .ok_or("Physical setup mirror instance is unavailable")?;
        let shared_construction = hardware
            .shared_construction
            .clone()
            .ok_or("Physical setup shared construction is unavailable")?;
        let labels = MirroredHalfLabels {
            source: Some(MirroredHalfLabel {
                name: Some(source.name.clone()),
                half: Some(source.half.clone()),
                role: Some(source.role.clone()),
            }),
            mirror: Some(MirroredHalfLabel {
                name: Some(mirror.name.clone()),
                half: Some(mirror.half.clone()),
                role: Some(mirror.role.clone()),
            }),
        };
        let mounting_hole = mounting_hole.ok_or_else(|| MISSING_MOUNTING_HOLE.to_string())?;
        Ok(Some((
            link.members.to_vec(),
            EditOperation::CreateMirroredHalf {
                pair_id: link.pair_id.clone(),
                source_instance_id: source.id.clone(),
                mode: MirroredHalfMode::ReuseBoard {
                    instance_id: mirror.id.clone(),
                },
                labels: Some(labels),
                physical_setup: Some(MirroredHalfPhysicalSetup {
                    topology: hardware.topology,
                    transport: hardware.transport,
                    shared_construction,
                    source_instance: source,
                }),
                companion_mode: Default::default(),
                companion_tweak: None,
                mapping: link.mapping,
                id_rule: MirroredIdRule::BoardPrefix,
                mounting_hole: Some(Box::new(mounting_hole.clone())),
            },
        )))
    }
}

const MISSING_MOUNTING_HOLE: &str =
    "A normalized mounting-hole definition is required to establish a mechanical pair.";

/// Catalogue-prepared resources for one physical-setup intent.
///
/// The proposal itself is recomputed against the accepted snapshot when Session
/// executes the intent, so preparation owns only the asynchronous catalogue work:
/// the normalized mounting-hole template Core needs when this setup establishes a
/// pair. `None` means the commit-time proposal established no pair.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedSetup {
    pub mounting_hole: Option<PartDefinition>,
}

/// Build a reference-compatible proposal from an immutable accepted document.
pub fn propose(
    accepted: &ProjectDoc,
    intent: SetupIntent,
    normalize: impl FnMut(&PartDefinition, bool) -> Result<(PartDefinition, bool), String>,
) -> Result<ProjectDoc, String> {
    propose_with_mirror_pair(accepted, intent, normalize).map(|proposal| proposal.document)
}

/// Build a proposal that also names the mechanical mirror pair it establishes.
///
/// `propose` is the document-only view of this function for callers that do not need
/// the link. The pair is returned as typed change data, never applied here.
pub fn propose_with_mirror_pair(
    accepted: &ProjectDoc,
    intent: SetupIntent,
    mut normalize: impl FnMut(&PartDefinition, bool) -> Result<(PartDefinition, bool), String>,
) -> Result<PhysicalSetupProposal, String> {
    match intent {
        SetupIntent::Topology {
            board_id,
            selected_instance_id,
            split,
            new_primary_id,
            new_secondary_id,
        } => {
            let (document, mirror_pair) = propose_topology(
                accepted,
                &board_id,
                selected_instance_id.as_deref(),
                split,
                &new_primary_id,
                &new_secondary_id,
            )?;
            Ok(PhysicalSetupProposal {
                document,
                mirror_pair,
            })
        }
        SetupIntent::LinkMirrorPair {
            pair_id,
            first_instance_id,
            second_instance_id,
            mapping,
            construction_source,
            correspondences,
        } => Ok(PhysicalSetupProposal {
            document: accepted.clone(),
            mirror_pair: propose_explicit_mirror_pair(
                accepted,
                &pair_id,
                &first_instance_id,
                &second_instance_id,
                mapping,
                construction_source.as_deref(),
                correspondences,
            )?,
        }),
        intent => Ok(PhysicalSetupProposal {
            document: propose_document(accepted, intent, &mut normalize)?,
            mirror_pair: None,
        }),
    }
}

fn propose_document(
    accepted: &ProjectDoc,
    intent: SetupIntent,
    normalize: &mut impl FnMut(&PartDefinition, bool) -> Result<(PartDefinition, bool), String>,
) -> Result<ProjectDoc, String> {
    match intent {
        SetupIntent::Topology {
            board_id,
            selected_instance_id,
            split,
            new_primary_id,
            new_secondary_id,
        } => propose_topology(
            accepted,
            &board_id,
            selected_instance_id.as_deref(),
            split,
            &new_primary_id,
            &new_secondary_id,
        )
        .map(|(document, _)| document),
        SetupIntent::Transport(transport) => propose_transport(accepted, transport),
        SetupIntent::ReversibleLayout(enabled) => propose_reversible(accepted, enabled, normalize),
        // The explicit link only proposes change data; `propose_with_mirror_pair`
        // returns it, so the document-only view is the accepted document.
        SetupIntent::LinkMirrorPair { .. } => Ok(accepted.clone()),
    }
}

fn propose_topology(
    accepted: &ProjectDoc,
    board_id: &str,
    selected_instance_id: Option<&str>,
    split: bool,
    new_primary_id: &str,
    new_secondary_id: &str,
) -> Result<(ProjectDoc, Option<MirrorPairLink>), String> {
    let old_hardware = accepted.hardware.as_ref();
    let current_topology = old_hardware
        .map(|hardware| hardware.topology)
        .unwrap_or_default();
    let target_topology = if split {
        HardwareTopology::Split
    } else {
        HardwareTopology::Unibody
    };
    if old_hardware.is_some() && current_topology == target_topology {
        if split
            && let Some(hardware) = old_hardware
            && !hardware.instances.iter().any(|instance| {
                hardware
                    .pairs
                    .iter()
                    .any(|pair| pair.members.iter().any(|member| member == &instance.id))
                    && instance.board_id == board_id
            })
            && let Some(primary) = hardware
                .instances
                .iter()
                .find(|instance| instance.board_id == board_id && instance.half == "left")
            && let Some(secondary) = hardware
                .instances
                .iter()
                .find(|instance| instance.board_id == board_id && instance.half == "right")
        {
            let primary_id = primary.id.clone();
            let secondary_id = secondary.id.clone();
            let mut proposed = accepted.clone();
            if let Some(proposed_hardware) = proposed.hardware.as_mut()
                && let Some(proposed_secondary) = proposed_hardware
                    .instances
                    .iter_mut()
                    .find(|instance| instance.id == secondary_id)
            {
                proposed_secondary.flipped = !primary.flipped;
            }
            proposed
                .parameters
                .insert("reversibleLayout".into(), Value::Bool(true));
            let mirror_pair = mirror_pair_link(
                &proposed,
                &format!("pair/{secondary_id}"),
                &primary_id,
                &secondary_id,
                None,
                None,
                None,
            )?;
            return Ok((proposed, mirror_pair));
        }
        return Ok((accepted.clone(), None));
    }
    // Choosing split creates a mirrored pair by default even when the existing
    // one-board setup persisted reversibleLayout=false.
    let reversible = split;

    let old_instances = old_hardware
        .map(|hardware| hardware.instances.as_slice())
        .unwrap_or_default();
    let retained = selected_instance_id
        .and_then(|id| old_instances.iter().find(|instance| instance.id == id))
        .or_else(|| {
            old_instances
                .iter()
                .find(|instance| instance.board_id == board_id)
        });
    let mechanical = retained
        .and_then(|instance| instance.mechanical.clone())
        .or_else(|| {
            accepted
                .mechanical
                .as_ref()
                .filter(|configuration| configuration.board_id == board_id)
                .cloned()
        });
    let mechanical = match mechanical {
        Some(configuration) => configuration,
        None => boardstudio_web_host::case_settings::initial_settings(accepted, board_id)?,
    };

    let mut hardware = old_hardware.cloned().unwrap_or_default();
    hardware.topology = target_topology;
    hardware.transport = if split {
        HardwareTransport::Wireless
    } else {
        HardwareTransport::None
    };
    hardware.shared_construction = Some(
        hardware
            .shared_construction
            .clone()
            .unwrap_or_else(|| mechanical.clone()),
    );

    let primary = if let Some(retained) = retained {
        let mut primary = retained.clone();
        primary.half = if split { "left" } else { "unibody" }.into();
        primary.role = if split { "central" } else { "standalone" }.into();
        if matches!(
            primary.name.as_str(),
            "Keyboard" | "Left half" | "Right half"
        ) {
            primary.name = if split { "Left half" } else { "Keyboard" }.into();
        }
        primary
    } else {
        ensure_fresh_id(new_primary_id, old_instances)?;
        make_instance(
            new_primary_id,
            board_id,
            if split { "left" } else { "unibody" },
            if split { "central" } else { "standalone" },
            if split { "Left half" } else { "Keyboard" },
            false,
            mechanical.clone(),
        )
    };

    let mut instances = vec![primary];
    let primary_id = instances[0].id.clone();
    let primary_flipped = instances[0].flipped;
    let primary_on_board = instances[0].board_id == board_id;
    if split {
        if retained.is_none() && new_primary_id == new_secondary_id {
            return Err("Physical instance IDs must be distinct".into());
        }
        ensure_fresh_id(new_secondary_id, old_instances)?;
        let flipped = reversible;
        // Opposite declared transforms on the target board make the two halves an
        // eligible mirror pair; an already-paired or off-board primary is not. An
        // eligible flipped second half keeps the authored feature clone so the
        // reviewed link can consume the shared mounts/openings; otherwise the legacy
        // clearing still applies.
        let already_paired = old_hardware.is_some_and(|hardware| {
            hardware
                .pairs
                .iter()
                .any(|pair| pair.members.iter().any(|member| member == &primary_id))
        });
        let eligible = primary_on_board
            && !already_paired
            && derived_mapping(primary_flipped, flipped).is_some();
        let mut secondary_mechanical = mechanical;
        if flipped && !eligible {
            secondary_mechanical.openings = Some(vec![]);
            secondary_mechanical.mounts.clear();
        }
        instances.push(make_instance(
            new_secondary_id,
            board_id,
            "right",
            "peripheral",
            "Right half",
            flipped,
            secondary_mechanical,
        ));
    }
    hardware.instances = instances;

    let mut proposed = accepted.clone();
    proposed.hardware = Some(hardware);
    if split {
        proposed
            .parameters
            .insert("reversibleLayout".into(), Value::Bool(true));
    }
    // The pair travels as typed change data for the Case adapter; the proposed
    // document never carries it, so it cannot be applied twice. A reused-PCB split
    // pairs the two halves only while both remain on the setup's board, and the map
    // is the physical reflection the halves' declared transforms imply: this setup
    // creates one board's two halves, so it never needs a translated/rotated map.
    let mirror_pair = if split && primary_on_board {
        mirror_pair_link(
            &proposed,
            &format!("pair/{new_secondary_id}"),
            &primary_id,
            new_secondary_id,
            None,
            None,
            None,
        )?
    } else {
        None
    };
    Ok((proposed, mirror_pair))
}

fn propose_transport(
    accepted: &ProjectDoc,
    transport: HardwareTransport,
) -> Result<ProjectDoc, String> {
    if !matches!(
        transport,
        HardwareTransport::Wired | HardwareTransport::Wireless
    ) {
        return Err("Split transport must be wired or wireless".into());
    }
    let mut proposed = accepted.clone();
    let hardware = proposed
        .hardware
        .as_mut()
        .filter(|hardware| hardware.topology == HardwareTopology::Split)
        .ok_or_else(|| "Split physical setup is unavailable".to_string())?;
    hardware.transport = transport;
    Ok(proposed)
}

fn propose_reversible(
    accepted: &ProjectDoc,
    enabled: bool,
    normalize: &mut impl FnMut(&PartDefinition, bool) -> Result<(PartDefinition, bool), String>,
) -> Result<ProjectDoc, String> {
    let mut proposed = accepted.clone();
    proposed
        .parameters
        .insert("reversibleLayout".into(), Value::Bool(enabled));
    let mut eligible = BTreeSet::new();
    proposed.definitions = accepted
        .definitions
        .iter()
        .map(|definition| {
            let (normalized, supports_reversible) = normalize(definition, enabled)?;
            if supports_reversible {
                eligible.insert(definition.id.clone());
            }
            Ok(normalized)
        })
        .collect::<Result<_, String>>()?;
    for part in &mut proposed.parts {
        if eligible.contains(&part.definition_id)
            && let Some(parameters) = part.generator_parameters.as_mut()
            && parameters.contains_key("reversible")
        {
            parameters.insert("reversible".into(), Value::Bool(enabled));
        }
    }
    if let Some(hardware) = proposed.hardware.as_mut() {
        for instance in &mut hardware.instances {
            instance.flipped = enabled && instance.half == "right";
        }
    }
    Ok(proposed)
}

fn ensure_fresh_id(id: &str, existing: &[PhysicalBoardInstance]) -> Result<(), String> {
    if id.is_empty() || existing.iter().any(|instance| instance.id == id) {
        return Err("Physical instance ID must be non-empty and unique".into());
    }
    Ok(())
}

/// The pair map two halves' declared board-local transforms imply.
///
/// Every [`PhysicalBoardInstance`] declares its Case frame in its board's document
/// frame: the identity when upright, or the board-local X reflection about zero when
/// `flipped`. The stored pair map is a `Pose2` composed with that fixed X reflection,
/// so it can represent the relative transform `T_second ∘ T_first^-1` only when the
/// two declared transforms differ. Opposite flips give the physical reflection
/// `q = (-p.x, p.y)` (`Pose2 { at: 0, rotation: 0 }`); equal flips give the identity,
/// which the pair model cannot represent, so those halves are not an eligible mirror
/// pair. The map is derived only from the halves' declared transforms: names, `half`
/// labels, `LayoutMirrorLink.axisX` and part coordinates never contribute.
fn derived_mapping(first_flipped: bool, second_flipped: bool) -> Option<Pose2> {
    (first_flipped != second_flipped).then(|| Pose2 {
        at: Vec2::default(),
        rotation: 0.0,
    })
}

/// The two halves a candidate link may join, with the map their declared transforms
/// imply and their resolved effective configurations.
struct EligibleHalves {
    /// `None` when the declared transforms cannot express the map, so a review of
    /// translated or rotated board frames must supply one explicitly.
    derived_mapping: Option<Pose2>,
    first: MechanicalConfiguration,
    second: MechanicalConfiguration,
}

/// The halves a candidate link may join, or `None` when the targets are ineligible.
///
/// Ineligible targets are the same or a missing instance, an instance whose board is
/// not in the document, an instance already owned by a pair, a taken pair id, or an
/// instance whose effective configuration cannot be resolved. Nothing here reads
/// names, `half` labels or part coordinates.
fn eligible_halves(
    document: &ProjectDoc,
    pair_id: &str,
    first_instance_id: &str,
    second_instance_id: &str,
) -> Option<EligibleHalves> {
    if first_instance_id == second_instance_id {
        return None;
    }
    let hardware = document.hardware.as_ref()?;
    if hardware.pairs.iter().any(|pair| pair.id == pair_id) {
        return None;
    }
    let first = hardware
        .instances
        .iter()
        .find(|instance| instance.id == first_instance_id)?;
    let second = hardware
        .instances
        .iter()
        .find(|instance| instance.id == second_instance_id)?;
    if !document
        .boards
        .iter()
        .any(|board| board.id == first.board_id)
        || !document
            .boards
            .iter()
            .any(|board| board.id == second.board_id)
    {
        return None;
    }
    if hardware.pairs.iter().any(|pair| {
        pair.members
            .iter()
            .any(|member| member == first_instance_id || member == second_instance_id)
    }) {
        return None;
    }
    // Comparing the halves' effective configuration does not read the accepted
    // contour projection.
    let first_inputs = effective_inputs(
        document,
        &[],
        &MechanicalTarget::Instance {
            instance_id: first_instance_id.into(),
        },
    )
    .ok()?;
    let second_inputs = effective_inputs(
        document,
        &[],
        &MechanicalTarget::Instance {
            instance_id: second_instance_id.into(),
        },
    )
    .ok()?;
    Some(EligibleHalves {
        derived_mapping: derived_mapping(first.flipped, second.flipped),
        first: first_inputs.configuration,
        second: second_inputs.configuration,
    })
}

/// The reviewable content of a candidate link between two existing halves.
#[derive(Clone, Debug, PartialEq)]
pub struct ReviewedLink {
    pub pair_id: String,
    pub members: [String; 2],
    /// The map the halves' declared transforms imply. `None` means they are not
    /// representable from the declared transforms alone, so the review must supply
    /// the map for a translated or rotated board frame.
    pub derived_mapping: Option<Pose2>,
    /// The construction source the review starts from; it may choose the other half.
    pub construction_source: String,
    /// The durable-id feature matches the link would consolidate. Features without a
    /// match stay local.
    pub correspondences: Vec<MechanicalFeatureCorrespondence>,
}

/// The candidate link a review shows before it is submitted, or `None` when the
/// targets are ineligible.
///
/// The review is computed from the accepted document alone: catalogue definitions are
/// not involved, the map comes from the halves' declared transforms (or must be
/// supplied), and the matches are durable feature ids. A blank pair id is malformed.
pub fn review_mirror_pair_link(
    document: &ProjectDoc,
    pair_id: &str,
    first_instance_id: &str,
    second_instance_id: &str,
) -> Result<Option<ReviewedLink>, String> {
    if pair_id.trim().is_empty() {
        return Err("A mechanical mirror pair needs a non-empty id".into());
    }
    let Some(halves) = eligible_halves(document, pair_id, first_instance_id, second_instance_id)
    else {
        return Ok(None);
    };
    Ok(Some(ReviewedLink {
        pair_id: pair_id.into(),
        members: [first_instance_id.into(), second_instance_id.into()],
        derived_mapping: halves.derived_mapping,
        construction_source: first_instance_id.into(),
        correspondences: correspondences_between(&halves.first, &halves.second),
    }))
}

/// The explicit link proposal for a setup intent that names two halves.
///
/// A blank pair id is a malformed request. A reviewed mapping that is not finite, a
/// reviewed construction source that is not a member, or a reviewed correspondence
/// naming a feature a half does not have are malformed too, so they are errors rather
/// than a silently different link.
fn propose_explicit_mirror_pair(
    accepted: &ProjectDoc,
    pair_id: &str,
    first_instance_id: &str,
    second_instance_id: &str,
    mapping: Option<Pose2>,
    construction_source: Option<&str>,
    correspondences: Option<Vec<MechanicalFeatureCorrespondence>>,
) -> Result<Option<MirrorPairLink>, String> {
    if pair_id.trim().is_empty() {
        return Err("A mechanical mirror pair needs a non-empty id".into());
    }
    mirror_pair_link(
        accepted,
        pair_id,
        first_instance_id,
        second_instance_id,
        mapping,
        construction_source,
        correspondences,
    )
}

/// The typed `LinkMirrorPair` payload for two eligible halves, or `None`.
///
/// A target is ineligible (and proposes nothing) when the halves are the same or
/// missing instance, an instance is already owned by a pair, an instance's board is
/// not in the document, the declared transforms do not imply a representable pair
/// map and the review supplied none, or either half's effective configuration cannot
/// be resolved. `pair_id` is never invented from a half label; callers pass the id
/// they captured at review. The reviewed mapping, construction source and feature
/// correspondences are carried as the review chose them; nothing is guessed from
/// coordinates, names, `half` labels or `flipped` flags.
fn mirror_pair_link(
    document: &ProjectDoc,
    pair_id: &str,
    first_instance_id: &str,
    second_instance_id: &str,
    reviewed_mapping: Option<Pose2>,
    reviewed_construction_source: Option<&str>,
    reviewed_correspondences: Option<Vec<MechanicalFeatureCorrespondence>>,
) -> Result<Option<MirrorPairLink>, String> {
    let Some(halves) = eligible_halves(document, pair_id, first_instance_id, second_instance_id)
    else {
        return Ok(None);
    };
    let mapping = match reviewed_mapping {
        Some(mapping) => {
            if !mapping.at.x.is_finite()
                || !mapping.at.y.is_finite()
                || !mapping.rotation.is_finite()
            {
                return Err("The mechanical pair mapping must be finite".into());
            }
            mapping
        }
        // Without a reviewed map only the halves' declared transforms can supply
        // one: opposite flips are the physical reflection, and equal flips are not
        // representable, so those halves have no eligible map.
        None => match halves.derived_mapping {
            Some(mapping) => mapping,
            None => return Ok(None),
        },
    };
    let construction_source = match reviewed_construction_source {
        Some(source) => {
            if source != first_instance_id && source != second_instance_id {
                return Err(
                    "The construction source must be one of the two physical instances".into(),
                );
            }
            source.to_string()
        }
        None => first_instance_id.to_string(),
    };
    let correspondences = match reviewed_correspondences {
        Some(correspondences) => {
            validate_correspondences(&halves.first, &halves.second, &correspondences)?;
            correspondences
        }
        None => correspondences_between(&halves.first, &halves.second),
    };
    Ok(Some(MirrorPairLink {
        pair_id: pair_id.into(),
        members: [first_instance_id.into(), second_instance_id.into()],
        mapping,
        construction_source,
        correspondences,
    }))
}

/// Rejects a reviewed feature match whose durable ids are not present on the halves.
///
/// The review selects the source geometry for each matched feature; a match naming a
/// feature a half does not have is malformed rather than a silently weaker link.
fn validate_correspondences(
    first: &MechanicalConfiguration,
    second: &MechanicalConfiguration,
    correspondences: &[MechanicalFeatureCorrespondence],
) -> Result<(), String> {
    for correspondence in correspondences {
        if find_feature(first, correspondence.kind, &correspondence.first_feature_id).is_none() {
            return Err(format!(
                "The first half has no {:?} feature {:?}",
                correspondence.kind, correspondence.first_feature_id
            ));
        }
        if find_feature(
            second,
            correspondence.kind,
            &correspondence.second_feature_id,
        )
        .is_none()
        {
            return Err(format!(
                "The second half has no {:?} feature {:?}",
                correspondence.kind, correspondence.second_feature_id
            ));
        }
    }
    Ok(())
}

fn find_feature(
    configuration: &MechanicalConfiguration,
    kind: MechanicalFeatureKind,
    feature_id: &str,
) -> Option<()> {
    match kind {
        MechanicalFeatureKind::Mount => configuration
            .mounts
            .iter()
            .any(|mount| mount.id == feature_id)
            .then_some(()),
        MechanicalFeatureKind::ClosureMount => configuration
            .closure_mounts
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|mount| mount.id == feature_id)
            .then_some(()),
        MechanicalFeatureKind::Opening => configuration
            .openings
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|opening| opening.id == feature_id)
            .then_some(()),
        MechanicalFeatureKind::GasketSupport => configuration
            .gasket_layout
            .as_ref()
            .map(|layout| layout.supports.as_slice())
            .unwrap_or_default()
            .iter()
            .any(|support| support.id == feature_id)
            .then_some(()),
    }
}

/// The matches between two halves' effective mechanical features.
///
/// Features match on their durable identity (mount/closure-mount/opening/anchor id),
/// never on their position. The first half's order is preserved, so the reviewed
/// default correspondence list is deterministic.
fn correspondences_between(
    first: &MechanicalConfiguration,
    second: &MechanicalConfiguration,
) -> Vec<MechanicalFeatureCorrespondence> {
    let mut correspondences = Vec::new();
    for mount in &first.mounts {
        if second.mounts.iter().any(|other| other.id == mount.id) {
            correspondences.push(correspondence(MechanicalFeatureKind::Mount, &mount.id));
        }
    }
    for mount in first.closure_mounts.as_deref().unwrap_or_default() {
        if second
            .closure_mounts
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|other| other.id == mount.id)
        {
            correspondences.push(correspondence(
                MechanicalFeatureKind::ClosureMount,
                &mount.id,
            ));
        }
    }
    for opening in first.openings.as_deref().unwrap_or_default() {
        if second
            .openings
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|other| other.id == opening.id)
        {
            correspondences.push(correspondence(MechanicalFeatureKind::Opening, &opening.id));
        }
    }
    for anchor in first
        .gasket_layout
        .as_ref()
        .map(|layout| layout.supports.as_slice())
        .unwrap_or_default()
    {
        if second
            .gasket_layout
            .as_ref()
            .map(|layout| layout.supports.as_slice())
            .unwrap_or_default()
            .iter()
            .any(|other| other.id == anchor.id)
        {
            correspondences.push(correspondence(
                MechanicalFeatureKind::GasketSupport,
                &anchor.id,
            ));
        }
    }
    correspondences
}

/// One setup-proposed match keeps the same durable id on both halves.
///
/// `None` keeps the legacy source: the pair's first member authors the shared value.
/// A reviewed link names the source per feature explicitly.
fn correspondence(kind: MechanicalFeatureKind, id: &str) -> MechanicalFeatureCorrespondence {
    MechanicalFeatureCorrespondence {
        kind,
        first_feature_id: id.into(),
        second_feature_id: id.into(),
        source_instance_id: None,
    }
}

fn make_instance(
    id: &str,
    board_id: &str,
    half: &str,
    role: &str,
    name: &str,
    flipped: bool,
    mechanical: MechanicalConfiguration,
) -> PhysicalBoardInstance {
    PhysicalBoardInstance {
        id: id.into(),
        name: name.into(),
        board_id: board_id.into(),
        half: half.into(),
        role: role.into(),
        flipped,
        controller_part_id: None,
        mechanical: Some(mechanical),
        // A setup creates new halves, so they own their configuration: the legacy shared
        // overlay and its first-instance fallback belong to instances the migration
        // marked, never to instances created now. An eligible pair link promotes both
        // members to the pair's shared construction afterwards.
        construction_ownership: MechanicalConstructionOwnership::Local,
        mechanical_identity_reservation: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::mechanical::change::MechanicalFeatureCorrespondence;
    use boardstudio_core::model::{
        Board, ElectricalBoardConfiguration, HardwareConfiguration, HardwareTopology,
        HardwareTransport, Layout, LayoutMirrorLink, MechanicalConfiguration, MechanicalMirrorPair,
        MechanicalOpening, MechanicalOpeningVertex, Mount, MountKind, PartDefinition,
        PhysicalBoardInstance, ProjectDoc,
    };
    use serde_json::{Value, json};

    fn document() -> ProjectDoc {
        let mut doc = ProjectDoc::empty("project", "Test project");
        doc.boards.push(Board {
            id: "board-a".into(),
            name: "Board A".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc
    }

    fn mechanical(doc: &ProjectDoc, thickness: f64) -> MechanicalConfiguration {
        let mut value =
            boardstudio_web_host::case_settings::initial_settings(doc, "board-a").unwrap();
        value.plate_thickness = thickness;
        value
    }

    fn instance(doc: &ProjectDoc, id: &str, half: &str, flipped: bool) -> PhysicalBoardInstance {
        PhysicalBoardInstance {
            id: id.into(),
            name: format!("{half} assembly"),
            board_id: "board-a".into(),
            half: half.into(),
            role: "custom-role".into(),
            flipped,
            controller_part_id: Some(format!("{id}-controller")),
            mechanical: Some(mechanical(doc, 4.2)),
            construction_ownership: MechanicalConstructionOwnership::Local,
            mechanical_identity_reservation: None,
        }
    }

    fn keep_definition(
        definition: &PartDefinition,
        _enabled: bool,
    ) -> Result<(PartDefinition, bool), String> {
        Ok((definition.clone(), false))
    }

    #[test]
    fn topology_preserves_flipped_primary_and_shared_construction_and_adds_reversible_secondary() {
        let mut doc = document();
        let primary = instance(&doc, "primary", "right", true);
        let primary_mechanical = primary.mechanical.clone();
        let shared = mechanical(&doc, 6.0);
        let electrical = ElectricalBoardConfiguration {
            board_id: "board-a".into(),
            controller_part_id: Some("electrical-u1".into()),
            ..Default::default()
        };
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport: HardwareTransport::None,
            instances: vec![primary.clone()],
            boards: vec![electrical.clone()],
            shared_construction: Some(shared.clone()),
            pairs: vec![],
        });
        doc.parameters.insert("unrelated".into(), json!("keep"));
        let accepted = doc.clone();

        let result = propose(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: Some("primary".into()),
                split: true,
                new_primary_id: "unused".into(),
                new_secondary_id: "right-new".into(),
            },
            keep_definition,
        )
        .unwrap();

        assert_eq!(doc, accepted, "accepted input remains immutable");
        let hardware = result.hardware.unwrap();
        assert_eq!(hardware.topology, HardwareTopology::Split);
        assert_eq!(hardware.transport, HardwareTransport::Wireless);
        assert_eq!(hardware.boards, vec![electrical]);
        assert_eq!(hardware.shared_construction, Some(shared));
        assert_eq!(hardware.instances[0].id, primary.id);
        assert!(
            hardware.instances[0].flipped,
            "topology preserves retained orientation"
        );
        assert_eq!(hardware.instances[0].mechanical, primary_mechanical);
        assert_eq!(
            hardware.instances[0].controller_part_id,
            primary.controller_part_id
        );
        assert_eq!(hardware.instances[1].id, "right-new");
        assert_eq!(hardware.instances[1].half, "right");
        assert!(
            hardware.instances[1].flipped,
            "new secondary follows effective reversible mode"
        );
        let new_mechanical = hardware.instances[1].mechanical.as_ref().unwrap();
        assert!(new_mechanical.openings.as_ref().is_none_or(Vec::is_empty));
        assert!(new_mechanical.mounts.is_empty());
        assert_eq!(result.parameters.get("unrelated"), Some(&json!("keep")));
    }

    #[test]
    fn a_fresh_setup_half_owns_its_configuration_instead_of_the_legacy_overlay() {
        // A brand-new project: no physical instances yet.
        let doc = document();
        let proposal = propose_with_mirror_pair(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: None,
                split: false,
                new_primary_id: "primary".into(),
                new_secondary_id: "unused".into(),
            },
            keep_definition,
        )
        .unwrap();
        let hardware = proposal.document.hardware.as_ref().expect("hardware");
        assert_eq!(hardware.instances.len(), 1);
        let first_half = &hardware.instances[0];
        assert_eq!(
            first_half.construction_ownership,
            MechanicalConstructionOwnership::Local,
            "a new unpaired half owns its own configuration, not the legacy overlay"
        );
        let own = first_half
            .mechanical
            .clone()
            .expect("the new half carries its configuration");

        // Its effective read is its own configuration: no legacy overlay and no
        // first-instance fallback can change what it shows.
        let resolved = |document: &ProjectDoc| {
            effective_inputs(
                document,
                &[],
                &MechanicalTarget::Instance {
                    instance_id: first_half.id.clone(),
                },
            )
            .expect("the new half resolves")
            .configuration
        };
        let before = resolved(&proposal.document);
        assert_eq!(
            before.wall_thickness, own.wall_thickness,
            "the new half reads back its own value"
        );

        // A shared-field write on the legacy source (where a legacy member's edit routes)
        // does not reach the new half.
        let mut legacy_member_edit = proposal.document.clone();
        legacy_member_edit
            .hardware
            .as_mut()
            .expect("hardware")
            .shared_construction
            .as_mut()
            .expect("the setup still writes a shared construction for existing halves")
            .wall_thickness = 9.0;
        assert_eq!(
            resolved(&legacy_member_edit).wall_thickness,
            own.wall_thickness,
            "a shared-field edit on a legacy member cannot reach a local half"
        );

        // A split whose retained primary sits on another board is never pair-linked
        // either, so the half it creates must still own its configuration.
        let mut off_board = document();
        off_board.boards.push(Board {
            id: "board-b".into(),
            name: "Board B".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let mut foreign = instance(&off_board, "foreign", "left", false);
        foreign.board_id = "board-b".into();
        off_board.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport: HardwareTransport::None,
            instances: vec![foreign],
            boards: vec![],
            shared_construction: None,
            pairs: vec![],
        });
        let split = propose_with_mirror_pair(
            &off_board,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: Some("foreign".into()),
                split: true,
                new_primary_id: "unused".into(),
                new_secondary_id: "secondary-new".into(),
            },
            keep_definition,
        )
        .unwrap();
        assert!(
            split.mirror_pair.is_none(),
            "an off-board primary is never pair-linked"
        );
        let created = split
            .document
            .hardware
            .as_ref()
            .expect("hardware")
            .instances
            .iter()
            .find(|instance| instance.id == "secondary-new")
            .expect("the created half");
        assert_eq!(
            created.construction_ownership,
            MechanicalConstructionOwnership::Local,
            "an unpaired created half owns its configuration even when the primary is off-board"
        );
    }

    #[test]
    fn topology_falls_back_to_board_matching_primary_and_resolved_mechanics_for_shared_construction()
     {
        let mut doc = document();
        let primary = instance(&doc, "board-primary", "left", false);
        let primary_mechanical = primary.mechanical.clone();
        let mut another_board = instance(&doc, "other", "unibody", false);
        another_board.board_id = "another-board".into();
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            transport: HardwareTransport::Wired,
            instances: vec![another_board, primary.clone()],
            boards: vec![],
            shared_construction: None,
            pairs: vec![],
        });
        let result = propose(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: None,
                split: false,
                new_primary_id: "unused".into(),
                new_secondary_id: "unused-right".into(),
            },
            keep_definition,
        )
        .unwrap();
        let hardware = result.hardware.unwrap();
        assert_eq!(hardware.instances.len(), 1);
        assert_eq!(hardware.instances[0].id, primary.id);
        assert_eq!(hardware.shared_construction, primary_mechanical);
        assert_eq!(hardware.transport, HardwareTransport::None);
    }

    #[test]
    fn new_physical_instances_use_the_existing_mechanical_defaults_factory() {
        let doc = document();
        let result = propose(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: None,
                split: true,
                new_primary_id: "primary-new".into(),
                new_secondary_id: "secondary-new".into(),
            },
            keep_definition,
        )
        .unwrap();
        let hardware = result.hardware.unwrap();
        let shared = hardware.shared_construction.as_ref().unwrap();
        assert_eq!(shared.board_id, "board-a");
        assert_eq!(shared.plate_thickness, 1.5);
        assert_eq!(hardware.instances[0].mechanical.as_ref(), Some(shared));
        assert_eq!(hardware.instances[1].mechanical.as_ref(), Some(shared));
        assert!(hardware.instances[1].flipped);
    }

    #[test]
    fn topology_uses_matching_canonical_board_mechanics_before_the_defaults_factory() {
        let mut doc = document();
        doc.mechanical = Some(mechanical(&doc, 7.25));
        let canonical = doc.mechanical.clone().unwrap();
        let result = propose(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: None,
                split: true,
                new_primary_id: "primary-new".into(),
                new_secondary_id: "secondary-new".into(),
            },
            keep_definition,
        )
        .unwrap();
        let hardware = result.hardware.unwrap();
        assert_eq!(hardware.shared_construction, Some(canonical.clone()));
        assert_eq!(hardware.instances[0].mechanical, Some(canonical.clone()));
        assert_eq!(hardware.instances[1].mechanical, Some(canonical));
    }

    #[test]
    fn transport_changes_only_the_transport_field() {
        let mut doc = document();
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            transport: HardwareTransport::Wireless,
            instances: vec![instance(&doc, "primary", "left", true)],
            boards: vec![ElectricalBoardConfiguration {
                board_id: "board-a".into(),
                ..Default::default()
            }],
            shared_construction: Some(mechanical(&doc, 5.0)),
            pairs: vec![],
        });
        let accepted = doc.clone();
        let result = propose(
            &doc,
            SetupIntent::Transport(HardwareTransport::Wired),
            keep_definition,
        )
        .unwrap();
        let mut expected = accepted.hardware.clone().unwrap();
        expected.transport = HardwareTransport::Wired;
        assert_eq!(result.hardware, Some(expected));
        assert_eq!(doc, accepted);
    }

    #[test]
    fn reversible_changes_document_flag_and_only_eligible_existing_part_overrides() {
        let mut doc = document();
        doc.definitions = vec![
            definition("supported", "ceoloide/switch_mx"),
            definition("custom", "vendor/custom"),
        ];
        doc.parts = vec![
            part(
                "explicit",
                "supported",
                json!({"reversible":false,"custom":9}),
            ),
            part("implicit", "supported", json!({"custom":7})),
            part("custom", "custom", json!({"reversible":false})),
        ];
        doc.hardware = Some(HardwareConfiguration {
            instances: vec![
                instance(&doc, "left", "left", true),
                instance(&doc, "right", "right", false),
            ],
            ..Default::default()
        });
        let accepted = doc.clone();
        let result = propose(
            &doc,
            SetupIntent::ReversibleLayout(true),
            |definition, enabled| {
                if definition.id == "supported" {
                    let mut result = definition.clone();
                    result
                        .generator
                        .as_mut()
                        .unwrap()
                        .parameters
                        .insert("reversible".into(), json!(enabled));
                    Ok((result, true))
                } else {
                    Ok((definition.clone(), false))
                }
            },
        )
        .unwrap();
        assert_eq!(doc, accepted, "accepted input remains immutable");
        assert_eq!(
            result.parameters.get("reversibleLayout"),
            Some(&Value::Bool(true))
        );
        assert_eq!(result.definitions[1], doc.definitions[1]);
        assert_eq!(
            result.parts[0]
                .generator_parameters
                .as_ref()
                .unwrap()
                .get("reversible"),
            Some(&json!(true))
        );
        assert_eq!(
            result.parts[0]
                .generator_parameters
                .as_ref()
                .unwrap()
                .get("custom"),
            Some(&json!(9))
        );
        assert_eq!(
            result.parts[1]
                .generator_parameters
                .as_ref()
                .unwrap()
                .get("reversible"),
            None
        );
        assert_eq!(
            result.parts[2]
                .generator_parameters
                .as_ref()
                .unwrap()
                .get("reversible"),
            Some(&json!(false))
        );
        assert!(!result.hardware.as_ref().unwrap().instances[0].flipped);
        assert!(result.hardware.as_ref().unwrap().instances[1].flipped);
    }

    /// The `reversible` flag the proposal writes into a part's parameters is a boolean, as the
    /// generator declares it, so Core's parameter check (ADR-0009) accepts the proposed document.
    #[test]
    fn reversible_writes_a_boolean_that_the_core_parameter_check_accepts() {
        let mut doc = document();
        doc.definitions = vec![definition("supported", "ceoloide/switch_mx")];
        doc.parts = vec![part("explicit", "supported", json!({"reversible": false}))];
        assert_eq!(boardstudio_core::generators::parameter_problems(&doc), []);
        let result = propose(
            &doc,
            SetupIntent::ReversibleLayout(true),
            |definition, _| Ok((definition.clone(), true)),
        )
        .unwrap();
        assert_eq!(
            boardstudio_core::generators::parameter_problems(&result),
            []
        );
        assert_eq!(
            result.parts[0].generator_parameters.as_ref().unwrap()["reversible"],
            json!(true)
        );
    }

    fn definition(id: &str, source: &str) -> PartDefinition {
        serde_json::from_value(json!({
            "id":id,"name":id,"kind":"switch","courtyard":[],"pads":[],
            "generator":{"source":source,"version":"1","parameters":{}}
        }))
        .unwrap()
    }

    fn part(id: &str, definition_id: &str, params: Value) -> boardstudio_core::model::Part {
        serde_json::from_value(json!({
            "id":id,"definitionId":definition_id,"reference":id,
            "pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front",
            "generatorParameters":params
        }))
        .unwrap()
    }

    #[test]
    fn topology_noop_and_invalid_transport_are_rejected_without_changing_input() {
        let mut doc = document();
        doc.hardware = Some(HardwareConfiguration::default());
        let accepted = doc.clone();
        let same = propose(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: None,
                split: false,
                new_primary_id: "primary".into(),
                new_secondary_id: "right".into(),
            },
            keep_definition,
        )
        .unwrap();
        assert_eq!(same, doc);
        assert!(
            propose(
                &doc,
                SetupIntent::Transport(HardwareTransport::Wired),
                keep_definition
            )
            .is_err()
        );
        assert_eq!(doc, accepted);
    }

    // -- Mechanical mirror-pair setup -------------------------------------------------

    fn mount(id: &str, x: f64, y: f64) -> Mount {
        Mount {
            id: id.into(),
            at: Vec2 { x, y },
            kind: MountKind::Hole,
            hole_diameter: 2.2,
            boss_diameter: None,
            height: None,
        }
    }

    fn opening(id: &str) -> MechanicalOpening {
        MechanicalOpening {
            id: id.into(),
            vertices: vec![
                MechanicalOpeningVertex {
                    id: format!("{id}-a"),
                    point: Vec2 { x: 0.0, y: 0.0 },
                },
                MechanicalOpeningVertex {
                    id: format!("{id}-b"),
                    point: Vec2 { x: 3.0, y: 0.0 },
                },
                MechanicalOpeningVertex {
                    id: format!("{id}-c"),
                    point: Vec2 { x: 1.0, y: 1.0 },
                },
            ],
            z: 1.0,
            height: 2.0,
            // Not zero, so a later vertex addition cannot mint an id these fixtures
            // already use. (Zero means "unset", and Core then reserves its own.)
            next_vertex_identity: 3,
        }
    }

    fn feature_instance(
        doc: &ProjectDoc,
        id: &str,
        board_id: &str,
        half: &str,
        flipped: bool,
        mounts: Vec<Mount>,
        openings: Vec<MechanicalOpening>,
    ) -> PhysicalBoardInstance {
        let mut configuration = mechanical(doc, 4.2);
        configuration.board_id = board_id.into();
        configuration.mounts = mounts;
        configuration.openings = Some(openings);
        PhysicalBoardInstance {
            id: id.into(),
            name: id.into(),
            board_id: board_id.into(),
            half: half.into(),
            role: "custom-role".into(),
            flipped,
            controller_part_id: None,
            mechanical: Some(configuration),
            construction_ownership: MechanicalConstructionOwnership::Local,
            mechanical_identity_reservation: None,
        }
    }

    /// A reviewed explicit link with no extra reviewed choice: the map comes from the
    /// halves' declared transforms and the construction source is the first member.
    fn link_intent(pair_id: &str, first: &str, second: &str) -> SetupIntent {
        SetupIntent::LinkMirrorPair {
            pair_id: pair_id.into(),
            first_instance_id: first.into(),
            second_instance_id: second.into(),
            mapping: None,
            construction_source: None,
            correspondences: None,
        }
    }

    fn mounting_hole_template() -> PartDefinition {
        serde_json::from_value(json!({
            "id":"generator:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
            "kind":"custom", "pads":[{"id":"pad-0", "number":"", "at":{"x":0,"y":0},
                "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
            "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
            "envelopeSource":{"courtyard":"generated"},
            "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
            "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
        }))
        .unwrap()
    }

    #[test]
    fn a_reviewed_mapping_links_distinct_board_halves_with_no_representable_declared_transform() {
        let mut doc = document();
        doc.boards.push(Board {
            id: "board-b".into(),
            name: "Board B".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        // Both halves are upright and scoped to different boards: the declared
        // transforms imply no map at all, so only a reviewed map can link them.
        let left = feature_instance(
            &doc,
            "left",
            "board-a",
            "left",
            false,
            vec![mount("mount-a", 2.0, 1.0)],
            vec![],
        );
        let right = feature_instance(
            &doc,
            "right",
            "board-b",
            "right",
            false,
            vec![mount("mount-b", 12.0, 4.0)],
            vec![],
        );
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            transport: HardwareTransport::None,
            instances: vec![left, right],
            boards: vec![],
            shared_construction: None,
            pairs: vec![],
        });
        let reviewed_mapping = Pose2 {
            at: Vec2 { x: 30.0, y: -5.0 },
            rotation: 180.0,
        };
        let reviewed = MechanicalFeatureCorrespondence {
            kind: MechanicalFeatureKind::Mount,
            first_feature_id: "mount-a".into(),
            second_feature_id: "mount-b".into(),
            source_instance_id: None,
        };
        let accepted = doc.clone();

        let proposal = propose_with_mirror_pair(
            &doc,
            SetupIntent::LinkMirrorPair {
                pair_id: "pair-reviewed".into(),
                first_instance_id: "left".into(),
                second_instance_id: "right".into(),
                mapping: Some(reviewed_mapping),
                construction_source: Some("right".into()),
                correspondences: Some(vec![reviewed.clone()]),
            },
            keep_definition,
        )
        .unwrap();

        assert_eq!(doc, accepted, "accepted input remains immutable");
        let link = proposal
            .mirror_pair
            .expect("an explicit reviewed map makes the halves eligible");
        assert_eq!(link.members, ["left".to_string(), "right".to_string()]);
        assert_eq!(link.mapping, reviewed_mapping, "the reviewed map is stored");
        assert_ne!(
            link.mapping,
            Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            "the review's translated/rotated map is not an inferred identity transform"
        );
        assert_eq!(link.construction_source, "right");
        assert_eq!(link.correspondences, vec![reviewed]);
    }

    #[test]
    fn a_malformed_reviewed_link_is_an_error_not_a_silently_different_pair() {
        let mut doc = document();
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            transport: HardwareTransport::None,
            instances: vec![
                feature_instance(
                    &doc,
                    "left",
                    "board-a",
                    "left",
                    false,
                    vec![mount("mount-1", 2.0, 1.0)],
                    vec![],
                ),
                feature_instance(
                    &doc,
                    "right",
                    "board-a",
                    "right",
                    true,
                    vec![mount("mount-1", -2.0, 1.0)],
                    vec![],
                ),
            ],
            boards: vec![],
            shared_construction: None,
            pairs: vec![],
        });

        let cases = [
            SetupIntent::LinkMirrorPair {
                pair_id: "pair-1".into(),
                first_instance_id: "left".into(),
                second_instance_id: "right".into(),
                mapping: Some(Pose2 {
                    at: Vec2 {
                        x: f64::NAN,
                        y: 0.0,
                    },
                    rotation: 0.0,
                }),
                construction_source: None,
                correspondences: None,
            },
            SetupIntent::LinkMirrorPair {
                pair_id: "pair-1".into(),
                first_instance_id: "left".into(),
                second_instance_id: "right".into(),
                mapping: None,
                construction_source: Some("other".into()),
                correspondences: None,
            },
            SetupIntent::LinkMirrorPair {
                pair_id: "pair-1".into(),
                first_instance_id: "left".into(),
                second_instance_id: "right".into(),
                mapping: None,
                construction_source: None,
                correspondences: Some(vec![MechanicalFeatureCorrespondence {
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-missing".into(),
                    second_feature_id: "mount-1".into(),
                    source_instance_id: None,
                }]),
            },
        ];
        for intent in cases {
            let outcome = propose_with_mirror_pair(&doc, intent.clone(), keep_definition);
            assert!(
                outcome.is_err(),
                "a malformed reviewed link must not propose {intent:?}"
            );
        }
    }

    #[test]
    fn a_reviewed_candidate_link_lists_the_derived_map_and_durable_matches() {
        let doc = document();
        let hardware = HardwareConfiguration {
            instances: vec![
                feature_instance(
                    &doc,
                    "left",
                    "board-a",
                    "left",
                    false,
                    vec![mount("mount-1", 2.0, 1.0)],
                    vec![opening("opening-1")],
                ),
                feature_instance(
                    &doc,
                    "right",
                    "board-a",
                    "right",
                    true,
                    vec![mount("mount-1", -2.0, 1.0)],
                    vec![opening("opening-1")],
                ),
            ],
            ..Default::default()
        };
        let mut with_hardware = doc.clone();
        with_hardware.hardware = Some(hardware);

        let review = review_mirror_pair_link(&with_hardware, "pair-1", "left", "right")
            .unwrap()
            .expect("opposite-flip halves have a reviewable candidate link");
        assert_eq!(review.members, ["left".to_string(), "right".to_string()]);
        assert_eq!(
            review.derived_mapping,
            Some(Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            }),
            "the declared transforms imply the physical reflection"
        );
        assert_eq!(review.construction_source, "left");
        assert_eq!(
            review.correspondences,
            vec![
                MechanicalFeatureCorrespondence {
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1".into(),
                    second_feature_id: "mount-1".into(),
                    source_instance_id: None,
                },
                MechanicalFeatureCorrespondence {
                    kind: MechanicalFeatureKind::Opening,
                    first_feature_id: "opening-1".into(),
                    second_feature_id: "opening-1".into(),
                    source_instance_id: None,
                },
            ],
            "the candidate matches durable feature ids only"
        );
        assert!(
            with_hardware.hardware.as_ref().unwrap().pairs.is_empty(),
            "a review never establishes the pair"
        );
    }

    #[test]
    fn a_reviewed_candidate_for_translated_board_frames_still_lists_matches_without_a_map() {
        let mut doc = document();
        doc.boards.push(Board {
            id: "board-b".into(),
            name: "Board B".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            transport: HardwareTransport::None,
            instances: vec![
                feature_instance(
                    &doc,
                    "left",
                    "board-a",
                    "left",
                    false,
                    vec![mount("mount-shared", 2.0, 1.0)],
                    vec![],
                ),
                feature_instance(
                    &doc,
                    "right",
                    "board-b",
                    "right",
                    false,
                    vec![mount("mount-shared", 12.0, 4.0)],
                    vec![],
                ),
            ],
            boards: vec![],
            shared_construction: None,
            pairs: vec![],
        });

        let review = review_mirror_pair_link(&doc, "pair-1", "left", "right")
            .unwrap()
            .expect("distinct boards with no declared transform are still reviewable");
        assert_eq!(
            review.derived_mapping, None,
            "an upright translated frame needs the review to supply its map"
        );
        assert_eq!(
            review.correspondences.len(),
            1,
            "the durable-id matches are still shown so the review can choose"
        );
    }

    #[test]
    fn a_reviewed_candidate_is_absent_for_ineligible_targets_and_blank_ids_are_malformed() {
        let doc = document();
        let hardware = HardwareConfiguration {
            instances: vec![
                feature_instance(
                    &doc,
                    "left",
                    "board-a",
                    "left",
                    false,
                    vec![mount("mount-1", 2.0, 1.0)],
                    vec![],
                ),
                feature_instance(
                    &doc,
                    "right",
                    "board-a",
                    "right",
                    true,
                    vec![mount("mount-1", -2.0, 1.0)],
                    vec![],
                ),
            ],
            ..Default::default()
        };
        let mut with_hardware = doc.clone();
        with_hardware.hardware = Some(hardware);

        assert!(
            review_mirror_pair_link(&with_hardware, "pair-1", "left", "left")
                .unwrap()
                .is_none(),
            "one half cannot link with itself"
        );
        assert!(
            review_mirror_pair_link(&with_hardware, "pair-1", "left", "missing")
                .unwrap()
                .is_none(),
            "a missing half is ineligible"
        );
        assert!(
            review_mirror_pair_link(&doc, "pair-1", "left", "right")
                .unwrap()
                .is_none(),
            "a document without physical instances has no candidate"
        );
        with_hardware.hardware.as_mut().unwrap().pairs = vec![stored_pair(
            &with_hardware,
            "pair-existing",
            "left",
            "right",
        )];
        assert!(
            review_mirror_pair_link(&with_hardware, "pair-new", "left", "right")
                .unwrap()
                .is_none(),
            "an instance already owned by a pair is ineligible"
        );
        assert!(
            review_mirror_pair_link(&with_hardware, "  ", "left", "right").is_err(),
            "a blank pair id is malformed, not an ineligible target"
        );
    }

    #[test]
    fn an_eligible_setup_proposal_establishes_its_pair_on_the_proposed_document() {
        let mut doc = document();
        doc.parameters
            .insert("reversibleLayout".into(), Value::Bool(true));
        let mut settings = mechanical(&doc, 4.2);
        settings.mounts = vec![mount("mount-1", 12.0, 3.0)];
        settings.openings = Some(vec![opening("opening-1")]);
        doc.mechanical = Some(settings);
        let accepted = doc.clone();

        let proposal = propose_with_mirror_pair(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: None,
                split: true,
                new_primary_id: "left-new".into(),
                new_secondary_id: "right-new".into(),
            },
            keep_definition,
        )
        .unwrap();
        assert_eq!(doc, accepted, "accepted input remains immutable");
        let link = proposal
            .mirror_pair
            .as_ref()
            .expect("opposite-flip halves propose a mechanical pair");
        assert_eq!(link.pair_id, "pair/right-new");
        assert_eq!(
            link.members,
            ["left-new".to_string(), "right-new".to_string()]
        );
        assert_eq!(link.construction_source, "left-new");
        // The mapping is the physical board-local X reflection about zero.
        assert_eq!(
            link.mapping,
            Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            }
        );
        // An asymmetric point so a symmetric fixture cannot hide a double reflection.
        let point = Vec2 { x: 4.5, y: -1.25 };
        let mapped = pair::forward(link.mapping, point);
        assert_eq!(mapped, Vec2 { x: -4.5, y: -1.25 });
        assert_ne!(mapped, point, "the mapping is not the identity");
        assert_eq!(pair::inverse(link.mapping, mapped), point, "round-trip");
        assert_eq!(
            link.correspondences,
            vec![
                MechanicalFeatureCorrespondence {
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1".into(),
                    second_feature_id: "mount-1".into(),
                    source_instance_id: None,
                },
                MechanicalFeatureCorrespondence {
                    kind: MechanicalFeatureKind::Opening,
                    first_feature_id: "opening-1".into(),
                    second_feature_id: "opening-1".into(),
                    source_instance_id: None,
                },
            ],
            "shared mounts and openings are covered by durable id"
        );
        let proposal_hardware = proposal.document.hardware.as_ref().unwrap();
        assert!(!proposal_hardware.instances[0].flipped);
        assert!(
            proposal_hardware.instances[1].flipped,
            "the reversible second half is flipped"
        );
        assert_eq!(
            proposal_hardware.instances[1]
                .mechanical
                .as_ref()
                .unwrap()
                .mounts,
            proposal_hardware.instances[0]
                .mechanical
                .as_ref()
                .unwrap()
                .mounts,
            "the eligible second half keeps its authored clone for the reviewed link"
        );
        assert!(
            proposal_hardware.pairs.is_empty(),
            "the proposal document alone never carries the pair"
        );
        assert!(
            proposal_hardware
                .instances
                .iter()
                .all(|instance| instance.construction_ownership
                    == MechanicalConstructionOwnership::Local),
            "the new halves start owning their own configuration, before the link \
             promotes them to the pair"
        );

        let document = submit_physical_setup(&doc, &proposal)
            .expect("the eligible pair is established through Core");

        let hardware = document.hardware.as_ref().unwrap();
        assert_eq!(hardware.instances.len(), 2);
        assert_eq!(
            hardware.pairs.len(),
            1,
            "the new halves and their pair land in one submission"
        );
        let pair = &hardware.pairs[0];
        assert_eq!(pair.id, "pair/right-new");
        assert_eq!(
            pair.members,
            ["left-new".to_string(), "right-new".to_string()]
        );
        assert_eq!(
            pair.construction,
            boardstudio_core::mechanical::features::shared_construction(&mechanical(&doc, 4.2)),
            "Core derives the pair's shared construction from the reviewed source"
        );
        assert!(
            pair.features
                .iter()
                .any(|feature| feature.kind == MechanicalFeatureKind::Mount),
            "the shared mount is linked"
        );
        let ownership = |id: &str| {
            hardware
                .instances
                .iter()
                .find(|instance| instance.id == id)
                .unwrap()
                .construction_ownership
        };
        assert_eq!(ownership("left-new"), MechanicalConstructionOwnership::Pair);
        assert_eq!(
            ownership("right-new"),
            MechanicalConstructionOwnership::Pair
        );
        // The linked value is stored once, with its author; the counterpart's visible
        // geometry is derived by Core's effective read rather than persisted twice.
        let feature = pair
            .features
            .iter()
            .find(|feature| feature.kind == MechanicalFeatureKind::Mount)
            .expect("the shared mount is linked");
        assert_eq!(feature.author, "left-new");
        assert_eq!(
            feature.value,
            boardstudio_core::model::MechanicalFeatureValue::Mount {
                mount: mount("mount-1", 12.0, 3.0),
            }
        );
        let right = boardstudio_core::mechanical::effective_inputs(
            &document,
            &[],
            &MechanicalTarget::Instance {
                instance_id: "right-new".into(),
            },
        )
        .expect("the mirrored half stays resolvable");
        assert_eq!(
            right.configuration.mounts[0].at,
            Vec2 { x: -12.0, y: 3.0 },
            "the counterpart mount is the reflection of the authored one"
        );
    }

    #[test]
    fn a_setup_proposal_without_a_pair_needs_no_mounting_hole() {
        // Equal declared transforms imply no representable map, so this split has no
        // eligible pair and therefore needs no mounting-hole template.
        let mut doc = document();
        let settings = mechanical(&doc, 4.2);
        doc.mechanical = Some(settings.clone());
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport: HardwareTransport::None,
            instances: vec![make_instance(
                "primary",
                "board-a",
                "unibody",
                "standalone",
                "Keyboard",
                true,
                settings,
            )],
            boards: vec![],
            shared_construction: None,
            pairs: vec![],
        });
        let proposal = propose_with_mirror_pair(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: None,
                split: true,
                new_primary_id: "left-new".into(),
                new_secondary_id: "right-new".into(),
            },
            keep_definition,
        )
        .unwrap();
        assert!(proposal.mirror_pair.is_none());
        let expected = proposal.document.clone();
        assert!(proposal.into_mirrored_operation(None).unwrap().is_none());
        assert_eq!(proposal.document, expected);
    }

    #[test]
    fn a_pair_application_failure_aborts_the_whole_proposal() {
        let mut doc = document();
        doc.parameters
            .insert("reversibleLayout".into(), Value::Bool(true));
        let mut settings = mechanical(&doc, 4.2);
        settings.mounts = vec![mount("mount-1", 12.0, 3.0)];
        doc.mechanical = Some(settings);
        let proposal = propose_with_mirror_pair(
            &doc,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: None,
                split: true,
                new_primary_id: "left-new".into(),
                new_secondary_id: "right-new".into(),
            },
            keep_definition,
        )
        .unwrap();
        assert!(proposal.mirror_pair.is_some());
        assert!(
            proposal.into_mirrored_operation(None).is_err(),
            "a pair that needs projection without its template publishes nothing"
        );
    }

    fn stored_pair(doc: &ProjectDoc, id: &str, first: &str, second: &str) -> MechanicalMirrorPair {
        MechanicalMirrorPair {
            id: id.into(),
            members: [first.into(), second.into()],
            mapping: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            construction: boardstudio_core::mechanical::features::shared_construction(&mechanical(
                doc, 4.2,
            )),
            construction_source: None,
            halves: None,
            features: vec![],
        }
    }

    #[test]
    fn distinct_pcb_halves_propose_an_explicit_non_identity_mapping_scoped_to_their_boards() {
        let mut doc = document();
        doc.boards.push(Board {
            id: "board-b".into(),
            name: "Board B".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let left = feature_instance(
            &doc,
            "left",
            "board-a",
            "left",
            false,
            vec![
                mount("mount-shared", 2.0, 1.0),
                mount("mount-left", 8.0, 1.0),
            ],
            vec![opening("opening-shared")],
        );
        let right = feature_instance(
            &doc,
            "right",
            "board-b",
            "right",
            true,
            vec![
                mount("mount-shared", -2.0, 1.0),
                mount("mount-right", -8.0, 1.0),
            ],
            vec![opening("opening-shared")],
        );
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            transport: HardwareTransport::None,
            instances: vec![left, right],
            boards: vec![],
            shared_construction: None,
            pairs: vec![],
        });
        let accepted = doc.clone();

        let proposal = propose_with_mirror_pair(
            &doc,
            link_intent("pair-1", "left", "right"),
            keep_definition,
        )
        .unwrap();

        assert_eq!(doc, accepted, "accepted input remains immutable");
        assert_eq!(
            proposal.document, accepted,
            "an explicit link never reassigns an instance's board"
        );
        let link = proposal
            .mirror_pair
            .expect("opposite-flip distinct halves propose a mechanical pair");
        assert_eq!(link.members, ["left".to_string(), "right".to_string()]);
        assert_eq!(link.construction_source, "left");
        // The declared map is the same board-local X reflection; it is non-identity
        // and is stored explicitly rather than read from names or an axis.
        assert_eq!(
            link.mapping,
            Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            }
        );
        let point = Vec2 { x: 2.75, y: 6.5 };
        let mapped = pair::forward(link.mapping, point);
        assert_ne!(mapped, point);
        assert_eq!(mapped, Vec2 { x: -2.75, y: 6.5 });
        assert_eq!(pair::inverse(link.mapping, mapped), point);
        assert_eq!(
            link.correspondences,
            vec![
                MechanicalFeatureCorrespondence {
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-shared".into(),
                    second_feature_id: "mount-shared".into(),
                    source_instance_id: None,
                },
                MechanicalFeatureCorrespondence {
                    kind: MechanicalFeatureKind::Opening,
                    first_feature_id: "opening-shared".into(),
                    second_feature_id: "opening-shared".into(),
                    source_instance_id: None,
                },
            ],
            "only the shared durable ids are matched, never part coordinates"
        );
        let hardware = proposal.document.hardware.as_ref().unwrap();
        assert_eq!(hardware.instances[0].board_id, "board-a");
        assert_eq!(hardware.instances[1].board_id, "board-b");
    }

    #[test]
    fn a_layout_only_pair_proposes_no_mechanical_pair_and_invents_no_instances() {
        let mut doc = document();
        doc.layouts.push(Layout {
            id: "left-layout".into(),
            name: "Left".into(),
            board_id: "board-a".into(),
            matrix_ids: vec!["matrix-a".into()],
            part_ids: vec![],
            mirror_link: None,
        });
        doc.layouts.push(Layout {
            id: "right-layout".into(),
            name: "Right".into(),
            board_id: "board-a".into(),
            matrix_ids: vec!["matrix-b".into()],
            part_ids: vec![],
            mirror_link: Some(LayoutMirrorLink {
                correspondences: vec![],
                exceptions: vec![],
                source_id: "left-layout".into(),
                axis_x: 12.0,
                split_gap: None,
            }),
        });
        let accepted = doc.clone();

        let proposal = propose_with_mirror_pair(
            &doc,
            link_intent("pair-1", "left", "right"),
            keep_definition,
        )
        .unwrap();

        assert_eq!(proposal.mirror_pair, None);
        assert_eq!(proposal.document, accepted);
        assert!(
            proposal.document.hardware.is_none(),
            "a layout-only pair never invents physical instances or splits a board"
        );
    }

    // -- Equivalence: physical setup against CreateMirroredHalf (ticket 26) ---------------

    /// The accepted starting document: one board with reversible layout on, and the unibody
    /// instance a user has before splitting, carrying the board's linked mount and opening.
    fn reversible_split_start() -> ProjectDoc {
        let mut doc = document();
        doc.parameters
            .insert("reversibleLayout".into(), Value::Bool(true));
        let mut settings = mechanical(&doc, 4.2);
        settings.mounts = vec![mount("mount-1", 12.0, 3.0)];
        settings.openings = Some(vec![opening("opening-1")]);
        settings.closure_mounts = Some(vec![
            mount("closure-1", 4.0, 20.0),
            mount("closure-2", 20.0, 4.0),
        ]);
        doc.mechanical = Some(settings.clone());
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport: HardwareTransport::None,
            instances: vec![make_instance(
                "left-new",
                "board-a",
                "unibody",
                "standalone",
                "Keyboard",
                false,
                settings,
            )],
            boards: vec![],
            shared_construction: None,
            pairs: vec![],
        });
        doc
    }

    /// Side A: the physical-setup split, reversible layout on, submitted through Core.
    fn physical_setup_split(start: &ProjectDoc) -> ProjectDoc {
        let proposal = propose_with_mirror_pair(
            start,
            SetupIntent::Topology {
                board_id: "board-a".into(),
                selected_instance_id: Some("left-new".into()),
                split: true,
                new_primary_id: "left-new".into(),
                new_secondary_id: "right-new".into(),
            },
            keep_definition,
        )
        .expect("the split proposal is accepted");
        submit_physical_setup(start, &proposal)
            .expect("the physical-setup pair is established through Core")
    }

    fn submit_physical_setup(
        start: &ProjectDoc,
        proposal: &PhysicalSetupProposal,
    ) -> Result<ProjectDoc, String> {
        let (target_ids, operation) = proposal
            .into_mirrored_operation(Some(&mounting_hole_template()))?
            .ok_or("physical setup did not produce a mirrored-half operation")?;
        let mut engine = CoreEngine::new();
        match engine.handle(CoreRequest::Open {
            id: "physical-setup-open".into(),
            document: start.clone(),
        }) {
            CoreReply::Scene { .. } => {}
            CoreReply::Error { message, .. } => return Err(message),
            _ => return Err("Core did not open the physical-setup input".into()),
        }
        let command = EditCommand {
            base_revision: start.revision,
            transaction_id: "physical-setup-create".into(),
            phase: EditPhase::Commit,
            target_ids,
            operation,
        };
        match engine.handle(CoreRequest::Edit {
            id: "physical-setup-create".into(),
            command,
        }) {
            CoreReply::Scene { document, .. } => Ok(*document),
            CoreReply::Error { message, .. } => Err(message),
            _ => Err("Core did not create the physical-setup pair".into()),
        }
    }

    /// Side B: the Core `CreateMirroredHalf` ReuseBoard operation on the same starting document,
    /// with the flip map, physical setup's labels and pair id, and the same mounting-hole template.
    fn reuse_board_half(start: &ProjectDoc) -> ProjectDoc {
        use boardstudio_core::model::{
            MirroredHalfLabel, MirroredHalfLabels, MirroredHalfMode, MirroredHalfPhysicalSetup,
            MirroredIdRule,
        };
        let source = start.hardware.as_ref().unwrap().instances[0].clone();
        let shared_construction = source.mechanical.clone().unwrap();
        let mut engine = CoreEngine::new();
        match engine.handle(CoreRequest::Open {
            id: "equivalence-open".into(),
            document: start.clone(),
        }) {
            CoreReply::Scene { .. } => {}
            other => panic!("Core did not open the starting document: {other:?}"),
        }
        let command = EditCommand {
            base_revision: start.revision,
            transaction_id: "equivalence-half".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::CreateMirroredHalf {
                pair_id: "pair/right-new".into(),
                source_instance_id: "left-new".into(),
                labels: Some(MirroredHalfLabels {
                    source: Some(MirroredHalfLabel {
                        name: Some("Left half".into()),
                        half: Some("left".into()),
                        role: Some("central".into()),
                    }),
                    mirror: Some(MirroredHalfLabel {
                        name: Some("Right half".into()),
                        half: Some("right".into()),
                        role: Some("peripheral".into()),
                    }),
                }),
                physical_setup: Some(MirroredHalfPhysicalSetup {
                    topology: HardwareTopology::Split,
                    transport: HardwareTransport::Wireless,
                    shared_construction,
                    source_instance: source,
                }),
                companion_mode: Default::default(),
                companion_tweak: None,
                mode: MirroredHalfMode::ReuseBoard {
                    instance_id: "right-new".into(),
                },
                mapping: Pose2 {
                    at: Vec2::default(),
                    rotation: 0.0,
                },
                id_rule: MirroredIdRule::BoardPrefix,
                mounting_hole: Some(Box::new(mounting_hole_template())),
            },
        };
        match engine.handle(CoreRequest::Edit {
            id: "equivalence-edit".into(),
            command,
        }) {
            CoreReply::Scene { document, .. } => *document,
            other => panic!("Core refused CreateMirroredHalf: {other:?}"),
        }
    }

    /// Every JSON leaf difference between two values, as (JSON pointer, A, B). A missing side
    /// is `None`. Arrays of equal length compare element by element; any other mismatch is one row.
    fn json_differences(
        path: &str,
        a: Option<&Value>,
        b: Option<&Value>,
        out: &mut Vec<(String, Option<Value>, Option<Value>)>,
    ) {
        match (a, b) {
            (Some(Value::Object(x)), Some(Value::Object(y))) => {
                let keys: BTreeSet<&String> = x.keys().chain(y.keys()).collect();
                for key in keys {
                    let escaped = key.replace('~', "~0").replace('/', "~1");
                    json_differences(&format!("{path}/{escaped}"), x.get(key), y.get(key), out);
                }
            }
            (Some(Value::Array(x)), Some(Value::Array(y))) if x.len() == y.len() => {
                for (index, (p, q)) in x.iter().zip(y).enumerate() {
                    json_differences(&format!("{path}/{index}"), Some(p), Some(q), out);
                }
            }
            _ if a == b => {}
            _ => out.push((path.into(), a.cloned(), b.cloned())),
        }
    }

    /// A short, comparable rendering of one side of a difference row.
    fn show(value: &Option<Value>) -> String {
        match value {
            None => "<absent>".into(),
            Some(Value::Object(_)) => "object".into(),
            Some(other) => other.to_string(),
        }
    }

    /// Ticket 26: on one accepted reversible split document, physical setup (A) and the Core
    /// `CreateMirroredHalf` ReuseBoard operation (B) must agree on everything the pair reflects.
    /// The differences that remain are pinned exactly, so any new or vanished difference fails
    /// with its JSON pointer. Ticket 32 supplies labels, resolving the six relabelling differences
    /// recorded in ticket 26's Outcome; the four other differences stay pinned.
    #[test]
    fn physical_setup_and_reuse_board_agree_on_a_reversible_split_pair() {
        let start = reversible_split_start();
        let a = serde_json::to_value(physical_setup_split(&start)).unwrap();
        let b = serde_json::to_value(reuse_board_half(&start)).unwrap();

        // Must be equal, and present (the comparison is not vacuous).
        for pointer in [
            "/hardware/instances/0/name",
            "/hardware/instances/0/half",
            "/hardware/instances/0/role",
            "/hardware/instances/1/name",
            "/hardware/instances/1/half",
            "/hardware/instances/1/role",
            "/hardware/pairs/0/id",
            "/hardware/pairs/0/members",
            "/hardware/pairs/0/mapping",
            "/hardware/pairs/0/constructionSource",
            "/hardware/pairs/0/construction",
            "/hardware/pairs/0/features",
            "/hardware/instances/1/flipped",
            "/hardware/instances/1/controllerPartId",
            "/hardware/instances/0/constructionOwnership",
            "/hardware/instances/1/constructionOwnership",
            "/hardware/instances/0/mechanical",
            "/hardware/instances/1/mechanical",
        ] {
            let left = a.pointer(pointer);
            assert!(left.is_some(), "A has no {pointer}");
            assert_eq!(left, b.pointer(pointer), "{pointer} differs");
        }
        let features = a.pointer("/hardware/pairs/0/features").unwrap();
        assert_eq!(
            features.as_array().map(Vec::len),
            Some(4),
            "the shared mount, opening and both closure mounts are linked: {features}"
        );
        let closure_parts = |doc: &Value| {
            doc["parts"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|part| part["id"].as_str().unwrap().starts_with("case-closure/"))
                .map(|part| part["id"].clone())
                .collect::<Vec<_>>()
        };
        assert!(
            !closure_parts(&a).is_empty(),
            "the generated case-closure parts exist on A"
        );
        assert_eq!(
            closure_parts(&a),
            closure_parts(&b),
            "the generated case-closure parts are the same"
        );

        // Every other difference, pinned exactly: (JSON pointer, A side, B side).
        let mut rows = Vec::new();
        json_differences("", Some(&a), Some(&b), &mut rows);
        let observed: Vec<(String, String, String)> = rows
            .iter()
            .map(|(path, left, right)| (path.clone(), show(left), show(right)))
            .collect();
        for (path, left, right) in &observed {
            eprintln!("EQUIVALENCE ROW {path} | A={left} | B={right}");
        }
        let expected: Vec<(String, String, String)> = vec![];
        assert_eq!(observed, expected, "the differences changed");
    }
}
