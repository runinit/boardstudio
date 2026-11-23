//! Private physical-setup proposal rules. This code prepares values only; it never submits edits.

#[cfg(test)]
use boardstudio_core::CoreEngine;
use boardstudio_core_mechanical::mechanical::MechanicalTarget;
use boardstudio_core_mechanical::mechanical::change::MechanicalFeatureCorrespondence;
use boardstudio_core_mechanical::mechanical::inputs::effective_inputs;
#[cfg(test)]
use boardstudio_core_mechanical::mechanical::pair;
#[cfg(test)]
use boardstudio_model::model::{CoreReply, CoreRequest, EditCommand, EditPhase};
use boardstudio_model::model::{
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

fn contains_id<T>(items: &[T], id_of: impl Fn(&T) -> &str, feature_id: &str) -> bool {
    items.iter().any(|item| id_of(item) == feature_id)
}

fn correspondences_for<T>(
    kind: MechanicalFeatureKind,
    first: &[T],
    second: &[T],
    id_of: impl Fn(&T) -> &str,
) -> Vec<MechanicalFeatureCorrespondence> {
    first
        .iter()
        .filter(|&item| contains_id(second, &id_of, id_of(item)))
        .map(|item| correspondence(kind, id_of(item)))
        .collect()
}

fn find_feature(
    configuration: &MechanicalConfiguration,
    kind: MechanicalFeatureKind,
    feature_id: &str,
) -> Option<()> {
    match kind {
        MechanicalFeatureKind::Mount => {
            contains_id(&configuration.mounts, |mount| mount.id.as_str(), feature_id)
        }
        MechanicalFeatureKind::ClosureMount => contains_id(
            configuration.closure_mounts.as_deref().unwrap_or_default(),
            |mount| mount.id.as_str(),
            feature_id,
        ),
        MechanicalFeatureKind::Opening => contains_id(
            configuration.openings.as_deref().unwrap_or_default(),
            |opening| opening.id.as_str(),
            feature_id,
        ),
        MechanicalFeatureKind::GasketSupport => contains_id(
            configuration
                .gasket_layout
                .as_ref()
                .map(|layout| layout.supports.as_slice())
                .unwrap_or_default(),
            |support| support.id.as_str(),
            feature_id,
        ),
    }
    .then_some(())
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
    let mut correspondences = correspondences_for(
        MechanicalFeatureKind::Mount,
        &first.mounts,
        &second.mounts,
        |mount| mount.id.as_str(),
    );
    correspondences.extend(correspondences_for(
        MechanicalFeatureKind::ClosureMount,
        first.closure_mounts.as_deref().unwrap_or_default(),
        second.closure_mounts.as_deref().unwrap_or_default(),
        |mount| mount.id.as_str(),
    ));
    correspondences.extend(correspondences_for(
        MechanicalFeatureKind::Opening,
        first.openings.as_deref().unwrap_or_default(),
        second.openings.as_deref().unwrap_or_default(),
        |opening| opening.id.as_str(),
    ));
    correspondences.extend(correspondences_for(
        MechanicalFeatureKind::GasketSupport,
        first
            .gasket_layout
            .as_ref()
            .map(|layout| layout.supports.as_slice())
            .unwrap_or_default(),
        second
            .gasket_layout
            .as_ref()
            .map(|layout| layout.supports.as_slice())
            .unwrap_or_default(),
        |support| support.id.as_str(),
    ));
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
mod tests;
