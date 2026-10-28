//! `CreateMirroredHalf`: Core creates the second half of a mechanical pair from its source
//! (ADR-0010). A new board receives a reflection of the source board through the pair's
//! `Pose2` map; a reused board gets a flipped instance and copies nothing.
//!
//! Everything is built on a clone of the document and committed only when the whole record
//! validates, so a refusal changes nothing. The changed ids are the new pair, both instances,
//! the new board, the ids the mechanical link and its generated clearance holes changed, and the
//! source and mirror id of every correspondence.
//!
//! What a new board does not receive, and why:
//!
//! - keymap bindings, the half's role and name, board references, copper and the
//!   protected wiring handoff: never reflected by nature (ADR-0010);
//! - `case-closure/*` clearance holes: generated from the pair's linked closure mounts;
//! - mounted modules: reflected by pose and id (ticket 27); a handed module is a finding, not a refusal;
//! - embedded circuits: reflected with their own namespace, `embedded/<mirror id>/...` (ticket 28);
//! - hardware specifications, critical fits and a battery: they name the source board's parts or
//!   one physical item, so the mirror starts without them.

use crate::matrix::layout;
use crate::mechanical::change::{
    MechanicalChange, MechanicalEdit, MechanicalFeatureCorrespondence,
};
use crate::mechanical::{self, MechanicalTarget, halves, inputs::effective_inputs, pair};
use crate::model::{
    Board, BoardOutline, ElectricalBoardConfiguration, Finding, HardwareConfiguration,
    HardwareTopology, HardwareTransport, Layout, MechanicalConfiguration,
    MechanicalConstructionOwnership, MechanicalFeatureKind, MirroredCompanionMode,
    MirroredCompanionTweak, MirroredCorrespondence, MirroredHalfLabel, MirroredHalfLabels,
    MirroredHalfMode, MirroredHalfPhysicalSetup, MirroredHalves, MirroredIdRule, MirroredItemKind,
    Net, OutlineBridge, OutlineConnection, OutlineControlPoint, OutlineFeature, OutlineProvenance,
    OutlineSettings, OutlineSnapshot, OutlineVersion, PartDefinition, PhysicalBoardInstance, Pin,
    Pose2, ProjectDoc, ProtectedOutlineGap, Scope, Severity, Vec2,
};
use crate::modules::{EmbeddedCircuit, MountedModule};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

mod derive;
mod inverse;
mod matrix_addition;
mod reconcile;
#[cfg(test)]
pub(crate) use reconcile::prepare_transaction;
pub(crate) use reconcile::{prepare_open, prepare_transaction_with_ledger};

/// Generated closure-clearance parts. They are regenerated from the pair, never copied.
const GENERATED_CLOSURE_PREFIX: &str = "case-closure/";

/// Matrix-owned parts attached to one board, shared by projection and inverse mapping.
fn matrix_owned_parts(document: &ProjectDoc, board_id: &str) -> BTreeSet<String> {
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return BTreeSet::new();
    };
    document
        .matrices
        .iter()
        .flat_map(|matrix| &matrix.part_ids)
        .filter(|part_id| board.part_ids.contains(part_id))
        .cloned()
        .collect()
}

/// Controller part ids on one board, shared by projection and outline anchoring.
///
/// Controllers keep the reflected heading (no half turn), so they follow the switch rule
/// everywhere matrix-owned parts do: pose projection and part-frame outline geometry.
fn controller_parts(document: &ProjectDoc, board_id: &str) -> BTreeSet<String> {
    let board_parts: BTreeSet<&str> = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .map(|board| board.part_ids.iter().map(String::as_str).collect())
        .unwrap_or_default();
    document
        .parts
        .iter()
        .filter(|part| board_parts.contains(part.id.as_str()))
        .filter(|part| derive::is_controller(&document.definitions, &part.definition_id))
        .map(|part| part.id.clone())
        .collect()
}

fn board_for_part<'a>(document: &'a ProjectDoc, part_id: &str) -> Option<&'a crate::model::Board> {
    document
        .boards
        .iter()
        .find(|board| board.part_ids.iter().any(|id| id == part_id))
}

/// What every mode of the operation shares.
struct Request<'a> {
    pair_id: &'a str,
    source_id: &'a str,
    mapping: Pose2,
    mounting_hole: Option<&'a PartDefinition>,
    companion_mode: MirroredCompanionMode,
    companion_tweak: Option<MirroredCompanionTweak>,
}

/// Creates the second half and returns the changed ids, sorted.
#[allow(clippy::too_many_arguments)] // Mirrors the typed operation fields at its single dispatch site.
pub(crate) fn create(
    doc: &mut ProjectDoc,
    pair_id: &str,
    source_instance_id: &str,
    mode: &MirroredHalfMode,
    mapping: Pose2,
    id_rule: MirroredIdRule,
    mounting_hole: Option<&PartDefinition>,
    companion_mode: MirroredCompanionMode,
    companion_tweak: Option<MirroredCompanionTweak>,
    labels: Option<&MirroredHalfLabels>,
) -> Result<Vec<String>, String> {
    let MirroredIdRule::BoardPrefix = id_rule;
    halves::validate_companion(companion_mode, companion_tweak)?;
    if !(mapping.at.x.is_finite() && mapping.at.y.is_finite() && mapping.rotation.is_finite()) {
        return Err("The mirrored half's mapping must be finite.".into());
    }
    let request = Request {
        pair_id,
        source_id: source_instance_id,
        mapping,
        mounting_hole,
        companion_mode,
        companion_tweak,
    };
    let mut next = doc.clone();
    let mut changed = match mode {
        MirroredHalfMode::ReuseBoard { instance_id } => {
            reuse_board(&mut next, &request, instance_id)?
        }
        MirroredHalfMode::NewBoard {
            board_id,
            instance_id,
        } => new_board(&mut next, &request, board_id, instance_id)?,
    };
    if let Some(labels) = labels {
        let mirror_id = match mode {
            MirroredHalfMode::ReuseBoard { instance_id }
            | MirroredHalfMode::NewBoard { instance_id, .. } => instance_id,
        };
        apply_labels(&mut next, pair_id, source_instance_id, mirror_id, labels)?;
    }
    layout::validate(&next)?;
    halves::validate_document(&next)?;
    changed.sort();
    changed.dedup();
    *doc = next;
    Ok(changed)
}

/// Applies the web physical-setup values and the pair on one draft, so any refusal leaves the
/// accepted document untouched.
#[allow(clippy::too_many_arguments)]
pub(crate) fn create_with_physical_setup(
    doc: &mut ProjectDoc,
    pair_id: &str,
    source_instance_id: &str,
    mode: &MirroredHalfMode,
    mapping: Pose2,
    id_rule: MirroredIdRule,
    mounting_hole: Option<&PartDefinition>,
    companion_mode: MirroredCompanionMode,
    companion_tweak: Option<MirroredCompanionTweak>,
    labels: Option<&MirroredHalfLabels>,
    setup: &MirroredHalfPhysicalSetup,
) -> Result<Vec<String>, String> {
    if setup.topology != HardwareTopology::Split
        || !matches!(
            setup.transport,
            HardwareTransport::Wired | HardwareTransport::Wireless
        )
    {
        return Err(
            "Mirrored physical setup requires split topology and wired or wireless transport."
                .into(),
        );
    }
    let source = &setup.source_instance;
    if source.id != source_instance_id
        || source.board_id != setup.shared_construction.board_id
        || !doc.boards.iter().any(|board| board.id == source.board_id)
    {
        return Err(
            "Physical setup source and shared construction must belong to a configured board."
                .into(),
        );
    }
    let mut next = doc.clone();
    let prior_ids = next
        .hardware
        .as_ref()
        .map(|hardware| {
            hardware
                .instances
                .iter()
                .map(|instance| instance.id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let hardware = next
        .hardware
        .get_or_insert_with(HardwareConfiguration::default);
    hardware.topology = setup.topology;
    hardware.transport = setup.transport;
    hardware.shared_construction = Some(setup.shared_construction.clone());
    hardware.instances = vec![source.clone()];
    let mut changed = create(
        &mut next,
        pair_id,
        source_instance_id,
        mode,
        mapping,
        id_rule,
        mounting_hole,
        companion_mode,
        companion_tweak,
        labels,
    )?;
    changed.extend(prior_ids);
    changed.sort();
    changed.dedup();
    *doc = next;
    Ok(changed)
}

/// Applies replacements to the draft after creation so omitted mirror fields keep the existing
/// derivation rule, even when the source is relabelled. A refusal discards the whole draft.
fn apply_labels(
    doc: &mut ProjectDoc,
    pair_id: &str,
    source_id: &str,
    mirror_id: &str,
    labels: &MirroredHalfLabels,
) -> Result<(), String> {
    let hardware = doc
        .hardware
        .as_mut()
        .ok_or("Physical instances are unavailable")?;
    for (id, side, label) in [
        (source_id, "source", labels.source.as_ref()),
        (mirror_id, "mirror", labels.mirror.as_ref()),
    ] {
        if let Some(label) = label {
            let instance = hardware
                .instances
                .iter_mut()
                .find(|instance| instance.id == id)
                .ok_or_else(|| format!("Physical instance {id:?} is unavailable."))?;
            apply_label(
                instance,
                label,
                &format!("mechanical pair {pair_id:?} instance {id:?} labels.{side}"),
            )?;
        }
    }
    let source = hardware
        .instances
        .iter()
        .find(|instance| instance.id == source_id)
        .ok_or("The source instance is unavailable")?;
    let mirror = hardware
        .instances
        .iter()
        .find(|instance| instance.id == mirror_id)
        .ok_or("The mirror instance is unavailable")?;
    if source.half == mirror.half && source.role == mirror.role {
        return Err(format!(
            "mechanical pair {pair_id:?} labels.mirror (instance {mirror_id:?}) claims the same half {:?} and role {:?} as labels.source (instance {source_id:?})",
            mirror.half, mirror.role
        ));
    }
    Ok(())
}

fn apply_label(
    instance: &mut PhysicalBoardInstance,
    label: &MirroredHalfLabel,
    location: &str,
) -> Result<(), String> {
    if let Some(name) = &label.name {
        if name.trim().is_empty() {
            return Err(format!("{location}.name must be non-empty"));
        }
        instance.name.clone_from(name);
    }
    if let Some(half) = &label.half {
        if !matches!(half.as_str(), "left" | "right" | "unibody") {
            return Err(format!(
                "{location}.half {half:?} must be left, right or unibody"
            ));
        }
        instance.half.clone_from(half);
    }
    if let Some(role) = &label.role {
        if !matches!(role.as_str(), "central" | "peripheral" | "standalone") {
            return Err(format!(
                "{location}.role {role:?} must be central, peripheral or standalone"
            ));
        }
        instance.role.clone_from(role);
    }
    Ok(())
}

/// The mirror id of a source id under the board-prefix rule: every whole `from` token becomes
/// `to`, and an id with no such token is prefixed with `to/`.
pub(crate) fn mirror_id(from: &str, to: &str, id: &str) -> String {
    substitute_token(id, from, to).unwrap_or_else(|| format!("{to}/{id}"))
}

/// Replaces each whole token equal to `from` (tokens are split at `/` and `-`), or `None` when
/// no token matched.
fn substitute_token(text: &str, from: &str, to: &str) -> Option<String> {
    substitute_delimited(text, &['/', '-'], |token| {
        (token == from).then(|| to.to_owned())
    })
}

/// Rewrites the tokens of `text` (split at any of `delimiters`) that `rename` answers for, or
/// `None` when no token was renamed.
fn substitute_delimited(
    text: &str,
    delimiters: &[char],
    rename: impl Fn(&str) -> Option<String>,
) -> Option<String> {
    let mut out = String::new();
    let mut matched = false;
    for piece in text.split_inclusive(delimiters) {
        let (token, delimiter) = match piece.chars().last() {
            Some(last) if delimiters.contains(&last) => (
                &piece[..piece.len() - last.len_utf8()],
                &piece[piece.len() - last.len_utf8()..],
            ),
            _ => (piece, ""),
        };
        match rename(token) {
            Some(renamed) => {
                out.push_str(&renamed);
                matched = true;
            }
            None => out.push_str(token),
        }
        out.push_str(delimiter);
    }
    matched.then_some(out)
}

/// A generated net name such as `PERIPHERAL_LEFT_TRRS_SPLIT-RX` names the part it wires; the
/// mirror's name carries the mirror board's token in the same case.
fn mirror_net_name(from: &str, to: &str, name: &str) -> String {
    substitute_delimited(name, &['_', '-', '/'], |token| {
        if token == from.to_uppercase() {
            Some(to.to_uppercase())
        } else if token == from {
            Some(to.to_owned())
        } else {
            None
        }
    })
    .unwrap_or_else(|| name.to_owned())
}

/// Footprints whose mirrored heading is settled, so they never raise the finding. Switches follow the
/// switch rule (ADR-0010, no half turn); diodes and mounting holes are symmetric.
const SYMMETRIC_FOOTPRINTS: [&str; 4] = [
    "ceoloide/switch_mx",
    "ceoloide/switch_choc_v1_v2",
    "ceoloide/diode_tht_sod123",
    "ceoloide/mounting_hole_npth",
];

/// Imported MX stabilisers: ADR-0010 keeps their world heading in every companion mode. Their
/// position takes the X reflection, and they never raise an uncertain-heading finding.
pub(crate) const SYMMETRIC_DEFINITIONS: [&str; 2] = [
    "kicad:marbastlib/STAB_MX_2u",
    "kicad:marbastlib/STAB_MX_6.25u",
];

pub(crate) fn companion_placement_disabled_reason(
    document: &ProjectDoc,
    pair_id: &str,
    item_id: &str,
) -> Option<&'static str> {
    let pair = document
        .hardware
        .as_ref()?
        .pairs
        .iter()
        .find(|pair| pair.id == pair_id)?;
    let correspondence = pair.halves.as_ref()?.correspondences.iter().find(|link| {
        link.kind == MirroredItemKind::Part
            && (link.source_id == item_id || link.mirror_id == item_id)
    })?;
    let source = document
        .parts
        .iter()
        .find(|part| part.id == correspondence.source_id)?;
    SYMMETRIC_DEFINITIONS
        .contains(&source.definition_id.as_str())
        .then_some(
            "Stabilisers keep their mirrored-X position and unchanged heading in every mode.",
        )
}

/// The uncertain-heading finding (ADR-0010, heading rule). A mirrored part of a pair whose footprint
/// is handed (any footprint in neither [`SYMMETRIC_FOOTPRINTS`] nor [`SYMMETRIC_DEFINITIONS`]) and whose heading is off the 90-degree
/// grid is reported for a person to confirm. Core neither refuses it nor turns the part to guess.
pub(crate) fn heading_findings(doc: &ProjectDoc) -> Vec<Finding> {
    let Some(hardware) = doc.hardware.as_ref() else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    for pair in &hardware.pairs {
        let Some(halves) = &pair.halves else {
            continue;
        };
        for correspondence in &halves.correspondences {
            if correspondence.kind != MirroredItemKind::Part {
                continue;
            }
            let Some(part) = doc
                .parts
                .iter()
                .find(|part| part.id == correspondence.mirror_id)
            else {
                continue;
            };
            let symmetric = SYMMETRIC_DEFINITIONS.contains(&part.definition_id.as_str())
                || doc
                    .definitions
                    .iter()
                    .find(|definition| definition.id == part.definition_id)
                    .and_then(|definition| definition.generator.as_ref())
                    .is_some_and(|generator| {
                        SYMMETRIC_FOOTPRINTS.contains(&generator.source.as_str())
                    });
            if symmetric || !part.pose.rotation.is_finite() {
                continue;
            }
            let off_grid = part.pose.rotation.rem_euclid(90.0);
            if off_grid.min(90.0 - off_grid) < 1e-9 {
                continue;
            }
            findings.push(Finding {
                id: format!("mirror/{}/heading-uncertain", part.id),
                severity: Severity::Warning,
                scope: Scope::Pcb,
                message: format!(
                    "The mirrored {} is handed and sits at {:.1} degrees, off the 90-degree grid. Confirm its pads on the mirrored half.",
                    part.reference, part.pose.rotation
                ),
                target_ids: vec![part.id.clone()],
            });
        }
    }
    findings
}

/// The handed-module finding (ADR-0010 amendment, modules). A mirrored module keeps its own geometry
/// and takes the reflected pose, so it is the mirror image of its source only when that geometry is
/// symmetric about its heading axis. No module definition carries a symmetry mark, and the board
/// outline alone cannot show one (components and connectors are not in it), so every mirrored module
/// is reported as a Warning for a person to confirm. Core neither refuses it nor turns it.
pub(crate) fn module_findings(doc: &ProjectDoc) -> Vec<Finding> {
    let Some(hardware) = doc.hardware.as_ref() else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    for pair in &hardware.pairs {
        let Some(halves) = &pair.halves else {
            continue;
        };
        for correspondence in &halves.correspondences {
            if correspondence.kind != MirroredItemKind::Module
                || !doc
                    .modules
                    .iter()
                    .any(|module| module.id == correspondence.mirror_id)
            {
                continue;
            }
            findings.push(Finding {
                id: format!("mirror/{}/handed-module", correspondence.mirror_id),
                severity: Severity::Warning,
                scope: Scope::Pcb,
                message: format!(
                    "The mirrored module {} keeps its own geometry at the reflected pose, so it is handed. Confirm its orientation and connector on the mirrored half.",
                    correspondence.mirror_id
                ),
                target_ids: vec![correspondence.mirror_id.clone()],
            });
        }
    }
    findings
}

/// A reflected pose for an independent part (ADR-0010 amendment, the full heading rule): the point
/// through the pair map, and the heading turned by the map's reflection over the full circle.
///
/// The linear part of the map is `R(rotation) * diag(-1, 1)`, a reflection about the axis at angle
/// `90 + rotation / 2`. For a heading `r`, `forward_angle(map, r) = rotation - r` is the image of
/// the orientation of a line, which is only fixed modulo 180 degrees. A heading is a direction, so
/// its image is that line image turned by 180 degrees: `forward_angle + 180`. On an axis map
/// (`rotation == 0`) this is `180 - r`: a part facing left faces right on the mirror. The result is
/// kept in `[0, 360)`; the rule never reduces the heading modulo 180.
///
/// This is the module and generic-geometry rule. Part projection goes through [`derive::pose`],
/// which keeps the reflected heading without the half turn for controllers.
pub(crate) fn mirror_pose(mapping: Pose2, pose: Pose2) -> Pose2 {
    derive::pose(mapping, pose, false)
}

/// A test-only window onto `derive::companion_assembly`, the authority for companion placement,
/// so the constraint endpoint rules can be pinned to it. No production caller.
#[cfg(test)]
#[allow(clippy::too_many_arguments)] // Mirrors derive's input record field for field.
pub(crate) fn companion_placement(
    offset: crate::model::Vec2,
    rotation: Option<f64>,
    key: &crate::model::Part,
    companion: &crate::model::Part,
    mapping: Pose2,
    mode: MirroredCompanionMode,
    tweak: Option<MirroredCompanionTweak>,
    stabiliser: bool,
    fallback: bool,
) -> Result<(crate::model::Vec2, Option<f64>), &'static str> {
    derive::companion_assembly(derive::CompanionAssemblyInput {
        offset,
        rotation,
        key: Some(key),
        companion: Some(companion),
        mapping,
        mode,
        tweak,
        stabiliser,
        fallback,
    })
}

/// A point in a part's own frame keeps its place in that frame; only the handedness flips,
/// because the mirror part already carries the reflected position and heading.
fn mirror_local(point: Vec2) -> Vec2 {
    Vec2 {
        x: -point.x,
        y: point.y,
    }
}

/// A point of an outline anchored to a part, in the mirror part's frame. A key switch or companion
/// is reflected with [`mirror_local`], because its heading is `rotation - r`. An independent part
/// takes `180 - r` (ticket 30), which is the mirror's heading turned by 180 degrees, so the same
/// world point is `(x, -y)` in its frame. A rect's own rotation is `-angle` under either rule.
fn anchored_local(point: Vec2, full_rule: bool) -> Vec2 {
    if full_rule {
        Vec2 {
            x: point.x,
            y: -point.y,
        }
    } else {
        mirror_local(point)
    }
}

/// The mirror's stored configuration: the source's, for the board it is placed on.
///
/// A new board starts without the lists that name the source board's parts or one physical item
/// (hardware specifications, critical fits, a battery). Openings, mounts, closure mounts and
/// gasket supports are linked through the pair, so they reach the mirror reflected.
fn mirror_configuration(
    source: &MechanicalConfiguration,
    board_id: &str,
    new_board: bool,
) -> MechanicalConfiguration {
    let mut configuration = source.clone();
    configuration.board_id = board_id.into();
    if new_board {
        configuration.hardware = None;
        configuration.critical_fits = None;
        configuration.battery = None;
    }
    configuration
}

/// What a mounted module's mirror takes from the board mirror: the two boards and instances, the
/// pair map, and the source-to-mirror id maps of the board's parts and modules.
struct ModuleMirror<'a> {
    from: &'a str,
    to: &'a str,
    source_instance: &'a str,
    mirror_instance: &'a str,
    mapping: Pose2,
    parts: &'a BTreeMap<String, String>,
    /// Source parts whose mirror is Omitted. A module plugged into one keeps everything except
    /// its connection, which is cleared as deleting the connector clears it.
    omitted_parts: &'a BTreeSet<String>,
    modules: &'a BTreeMap<String, String>,
}

/// A mounted module on the mirror (ADR-0010 amendment, modules; ticket 27). The pose goes through the
/// pair map and the heading takes the full rule `rotation + 180 - r`, the independent-part rule. The
/// module is not flipped, so its faces, gap, attachment, supports, definition and every parameter are
/// unchanged. Its own id and bus id take the board-prefix rule, its connector and upstream module take
/// their mirror ids, and a pin to the source instance moves to the mirror instance. A reference the
/// mirror cannot follow refuses the record by name (the caller discards the whole edit).
fn mirror_module(module: &MountedModule, context: &ModuleMirror) -> Result<MountedModule, String> {
    let host_instance_id = match module.host_instance_id.as_deref() {
        None => None,
        Some(id) if id == context.source_instance => Some(context.mirror_instance.to_owned()),
        Some(other) => {
            return Err(format!(
                "Module {:?} is pinned to physical instance {other:?}, which is not the source {:?}; its mirror has no instance to be pinned to.",
                module.id, context.source_instance
            ));
        }
    };
    let pose = mirror_pose(
        context.mapping,
        Pose2 {
            at: module.at,
            rotation: module.rotation,
        },
    );
    let mut mirrored = module.clone();
    mirrored.id = context.modules[&module.id].clone();
    mirrored.host_board_id = context.to.into();
    mirrored.host_instance_id = host_instance_id;
    mirrored.at = pose.at;
    mirrored.rotation = pose.rotation;
    if mirrored.connection.as_ref().is_some_and(|connection| {
        context
            .omitted_parts
            .contains(&connection.host_connector_part_id)
    }) {
        mirrored.connection = None;
    }
    if let Some(connection) = mirrored.connection.as_mut() {
        let connector = connection.host_connector_part_id.clone();
        if !connector.is_empty() {
            connection.host_connector_part_id =
                context.parts.get(&connector).cloned().ok_or_else(|| {
                    format!(
                        "Module {:?} connects to {connector:?}, which is not a part of board {:?}.",
                        module.id, context.from
                    )
                })?;
        }
        connection.bus_id = mirror_id(context.from, context.to, &connection.bus_id);
        if let Some(upstream) = connection.upstream_module_id.clone() {
            connection.upstream_module_id = Some(context.modules.get(&upstream).cloned().ok_or_else(|| {
                format!(
                    "Module {:?} takes its output from {upstream:?}, which is not a module of board {:?}.",
                    module.id, context.from
                )
            })?);
        }
    }
    Ok(mirrored)
}

/// Every mechanical feature the configuration carries, matched to itself on the other half.
fn feature_correspondences(
    configuration: &MechanicalConfiguration,
) -> Vec<MechanicalFeatureCorrespondence> {
    let same = |kind: MechanicalFeatureKind, id: &str| MechanicalFeatureCorrespondence {
        kind,
        first_feature_id: id.into(),
        second_feature_id: id.into(),
        source_instance_id: None,
    };
    let mut features = Vec::new();
    features.extend(
        configuration
            .mounts
            .iter()
            .map(|mount| same(MechanicalFeatureKind::Mount, &mount.id)),
    );
    features.extend(
        configuration
            .closure_mounts
            .iter()
            .flatten()
            .map(|mount| same(MechanicalFeatureKind::ClosureMount, &mount.id)),
    );
    features.extend(
        configuration
            .openings
            .iter()
            .flatten()
            .map(|opening| same(MechanicalFeatureKind::Opening, &opening.id)),
    );
    features.extend(
        configuration
            .gasket_layout
            .iter()
            .flat_map(|layout| &layout.supports)
            .map(|support| same(MechanicalFeatureKind::GasketSupport, &support.id)),
    );
    features
}

fn source_half(
    doc: &ProjectDoc,
    request: &Request,
) -> Result<(PhysicalBoardInstance, MechanicalConfiguration), String> {
    let (pair_id, source_id) = (request.pair_id, request.source_id);
    let hardware = doc
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?;
    if pair_id.trim().is_empty() || hardware.pairs.iter().any(|pair| pair.id == pair_id) {
        return Err(format!(
            "A mechanical pair named {pair_id:?} already exists."
        ));
    }
    let instance = hardware
        .instances
        .iter()
        .find(|instance| instance.id == source_id)
        .ok_or_else(|| format!("Physical instance {source_id:?} is unavailable."))?
        .clone();
    if hardware
        .pairs
        .iter()
        .any(|pair| pair.members.iter().any(|member| member == source_id))
    {
        return Err(format!(
            "Physical instance {source_id:?} already belongs to a mechanical pair."
        ));
    }
    if !doc.boards.iter().any(|board| board.id == instance.board_id) {
        return Err(format!(
            "The board of physical instance {source_id:?} is unavailable."
        ));
    }
    let configuration = effective_inputs(
        doc,
        &[],
        &MechanicalTarget::Instance {
            instance_id: source_id.into(),
        },
    )
    .map_err(|error| {
        format!("Cannot create the mirrored half of {source_id:?}; set up its mechanical configuration first ({error}).")
    })?
    .configuration;
    Ok((instance, configuration))
}

/// Refuses a board that already names a half of a mirrored-half record.
fn ensure_board_unclaimed(doc: &ProjectDoc, board_id: &str) -> Result<(), String> {
    let Some(hardware) = doc.hardware.as_ref() else {
        return Ok(());
    };
    for pair in &hardware.pairs {
        if let Some(halves) = &pair.halves
            && (halves.source_board_id == board_id || halves.mirror_board_id == board_id)
        {
            return Err(format!(
                "Board {board_id:?} is already a half of mechanical pair {:?}.",
                pair.id
            ));
        }
    }
    Ok(())
}

fn ensure_instance_fresh(doc: &ProjectDoc, instance_id: &str) -> Result<(), String> {
    let exists = doc
        .hardware
        .as_ref()
        .is_some_and(|hardware| hardware.instances.iter().any(|i| i.id == instance_id));
    if instance_id.trim().is_empty() || exists {
        return Err(format!("Physical instance {instance_id:?} already exists."));
    }
    Ok(())
}

/// The mirror's half label: the opposite half of a left/right pair; other labels are kept.
fn mirror_half_label(source_half: &str) -> &str {
    match source_half {
        "left" => "right",
        "right" => "left",
        other => other,
    }
}

/// The second half's instance: the opposite half label with the recipes' central/peripheral
/// roles, owning its configuration until the link promotes both members to the pair.
fn mirror_instance(
    source: &PhysicalBoardInstance,
    id: &str,
    board_id: &str,
    flipped: bool,
    controller_part_id: Option<String>,
    configuration: MechanicalConfiguration,
) -> PhysicalBoardInstance {
    let half = mirror_half_label(&source.half).to_owned();
    PhysicalBoardInstance {
        id: id.into(),
        name: match half.as_str() {
            "left" => "Left half".into(),
            "right" => "Right half".into(),
            _ => format!("{} mirror", source.name),
        },
        board_id: board_id.into(),
        role: if half == "right" {
            "peripheral"
        } else {
            "central"
        }
        .into(),
        half,
        flipped,
        controller_part_id,
        mechanical: Some(configuration),
        construction_ownership: MechanicalConstructionOwnership::Local,
        mechanical_identity_reservation: None,
    }
}

/// Links the two members through the mechanical `LinkMirrorPair` operation: the pair, its
/// shared construction, every linked feature authored by the source, the generated clearance
/// holes, and the mirrored-half record. The mirror instance must already be in the document with
/// a configuration that carries the same feature ids as the source.
fn link_halves(
    doc: &mut ProjectDoc,
    request: &Request,
    mirror_id: &str,
    halves: MirroredHalves,
) -> Result<Vec<String>, String> {
    let mirror = effective_inputs(
        doc,
        &[],
        &MechanicalTarget::Instance {
            instance_id: mirror_id.into(),
        },
    )?
    .configuration;
    let edit = MechanicalEdit {
        target: MechanicalTarget::Instance {
            instance_id: request.source_id.into(),
        },
        change: MechanicalChange::LinkMirrorPair {
            pair_id: request.pair_id.into(),
            members: [request.source_id.into(), mirror_id.into()],
            mapping: request.mapping,
            construction_source: request.source_id.into(),
            correspondences: feature_correspondences(&mirror),
        },
        mounting_hole: request
            .mounting_hole
            .map(|definition| Box::new(definition.clone())),
    };
    let changed = mechanical::edit::apply(doc, &[], &edit)?;
    doc.hardware
        .as_mut()
        .and_then(|hardware| {
            hardware
                .pairs
                .iter_mut()
                .find(|pair| pair.id == request.pair_id)
        })
        .ok_or("The mechanical pair was not recorded.")?
        .halves = Some(halves);
    Ok(changed)
}

fn reuse_board(
    doc: &mut ProjectDoc,
    request: &Request,
    instance_id: &str,
) -> Result<Vec<String>, String> {
    if request.companion_tweak.is_some() {
        return Err("reuse-board copies no companions; companionTweak requires new-board".into());
    }
    let (source, configuration) = source_half(doc, request)?;
    if source.flipped {
        return Err(format!(
            "Physical instance {:?} is already flipped; a reused board needs an unflipped source.",
            request.source_id
        ));
    }
    // A flipped instance of the same board is the physical reflection `q = (-p.x, p.y)`, which is
    // the pair map with no translation and no turn; any other map describes a different part.
    if request.mapping.at.x != 0.0 || request.mapping.at.y != 0.0 || request.mapping.rotation != 0.0
    {
        return Err(format!(
            "A flipped instance of board {:?} is the plain reflection: its map must be zero \
             translation and zero rotation, not {:?}. Create a new board for any other map.",
            source.board_id, request.mapping
        ));
    }
    ensure_board_unclaimed(doc, &source.board_id)?;
    ensure_instance_fresh(doc, instance_id)?;
    // A module pinned to the source instance is not active on the flipped instance, so it would
    // vanish from the mirror. Copying it is not supported yet: refuse it by name.
    let pinned: Vec<&str> = doc
        .modules
        .iter()
        .filter(|module| {
            module.host_board_id == source.board_id
                && module.host_instance_id.as_deref() == Some(request.source_id)
        })
        .map(|module| module.id.as_str())
        .collect();
    if !pinned.is_empty() {
        return Err(format!(
            "Modules {} are pinned to physical instance {:?}; reuse-board cannot copy a pinned module yet. Unpin them or create a new board.",
            pinned.join(", "),
            request.source_id
        ));
    }
    // An embedded circuit belongs to the board, not to an instance (it has no instance pin), so the
    // flipped instance shows the circuit without a copy, as it shows the board's own parts.
    let board = source.board_id.clone();
    let mirror = mirror_instance(
        &source,
        instance_id,
        &board,
        true,
        None,
        mirror_configuration(&configuration, &board, false),
    );
    doc.hardware
        .as_mut()
        .ok_or("Physical instances are unavailable")?
        .instances
        .push(mirror);
    link_halves(
        doc,
        request,
        instance_id,
        MirroredHalves {
            companion_mode: request.companion_mode,
            source_board_id: board.clone(),
            mirror_board_id: board,
            correspondences: vec![],
            exceptions: vec![],
        },
    )
}

/// Every id a document already uses in the namespaces a mirror writes into.
fn taken_ids(doc: &ProjectDoc) -> BTreeSet<String> {
    let mut taken = BTreeSet::new();
    taken.extend(doc.boards.iter().map(|b| b.id.clone()));
    taken.extend(doc.parts.iter().map(|p| p.id.clone()));
    taken.extend(doc.modules.iter().map(|m| m.id.clone()));
    taken.extend(doc.matrices.iter().map(|m| m.id.clone()));
    taken.extend(doc.layouts.iter().map(|l| l.id.clone()));
    taken.extend(doc.outline.iter().map(|o| o.id().to_owned()));
    taken.extend(doc.nets.iter().map(|n| n.id.clone()));
    taken.extend(
        doc.board_outlines
            .iter()
            .flat_map(|outline| outline.versions.iter().map(|v| v.id.clone())),
    );
    taken
}

fn mint(taken: &mut BTreeSet<String>, id: String) -> Result<String, String> {
    if !taken.insert(id.clone()) {
        return Err(format!("The mirrored id {id:?} is already in use."));
    }
    Ok(id)
}

/// How a feature's ids are renamed. Outline identities share one document namespace, so every
/// identity the rename makes is recorded and checked once the whole outline is built.
struct Renames<'a> {
    /// Maps an outline identity (a feature, connection, gap or bridge) and records the result.
    identity: &'a dyn Fn(&str) -> String,
    /// Maps a reference to a part of the source board to its mirror; foreign ids stay as they are.
    part: &'a dyn Fn(&str) -> String,
    /// Whether a source part takes the full heading rule (an independent part, ticket 30) rather
    /// than the switch rule (a key or a companion). It decides how an outline anchored to it is
    /// reflected in the part's frame.
    full_rule_part: &'a dyn Fn(&str) -> bool,
}

fn mirror_control(
    point: &OutlineControlPoint,
    mapping: Pose2,
    renames: &Renames,
) -> OutlineControlPoint {
    match &point.part_id {
        // Attached to a part: the point is in that part's frame.
        Some(part) => OutlineControlPoint {
            at: anchored_local(point.at, (renames.full_rule_part)(part)),
            part_id: Some((renames.part)(part)),
        },
        None => OutlineControlPoint {
            at: pair::forward(mapping, point.at),
            part_id: None,
        },
    }
}

fn mirror_gap(gap: &ProtectedOutlineGap, mapping: Pose2, renames: &Renames) -> ProtectedOutlineGap {
    ProtectedOutlineGap {
        id: (renames.identity)(&gap.id),
        points: gap
            .points
            .iter()
            .map(|point| mirror_control(point, mapping, renames))
            .collect(),
    }
}

fn mirror_settings(
    settings: &OutlineSettings,
    mapping: Pose2,
    renames: &Renames,
) -> OutlineSettings {
    let mut mirrored = settings.clone();
    if let Some(repair) = mirrored.repair.as_mut() {
        repair.keep_gaps = repair
            .keep_gaps
            .iter()
            .map(|gap| mirror_gap(gap, mapping, renames))
            .collect();
    }
    mirrored
}

/// Reflects a feature. A point in world coordinates goes through the pair map; a point anchored
/// to a part is in that part's frame and only flips handedness. A reflected polygon keeps its
/// orientation by reversing its point order.
fn mirror_feature(feature: &OutlineFeature, mapping: Pose2, renames: &Renames) -> OutlineFeature {
    match feature {
        OutlineFeature::Polygon {
            anchor_part_id,
            id,
            points,
            operation,
        } => OutlineFeature::Polygon {
            anchor_part_id: anchor_part_id.as_deref().map(renames.part),
            id: (renames.identity)(id),
            points: points
                .iter()
                .rev()
                .map(|point| match anchor_part_id.as_deref() {
                    Some(anchor) => anchored_local(*point, (renames.full_rule_part)(anchor)),
                    None => pair::forward(mapping, *point),
                })
                .collect(),
            operation: *operation,
        },
        OutlineFeature::Rect {
            rotation,
            anchor_part_id,
            id,
            center,
            size,
            radius,
            operation,
        } => OutlineFeature::Rect {
            rotation: if anchor_part_id.is_some() {
                rotation.map(|angle| -angle)
            } else {
                (rotation.is_some() || mapping.rotation != 0.0)
                    .then(|| pair::forward_angle(mapping, rotation.unwrap_or(0.0)))
            },
            anchor_part_id: anchor_part_id.as_deref().map(renames.part),
            id: (renames.identity)(id),
            center: match anchor_part_id.as_deref() {
                Some(anchor) => anchored_local(*center, (renames.full_rule_part)(anchor)),
                None => pair::forward(mapping, *center),
            },
            size: *size,
            radius: *radius,
            operation: *operation,
        },
        OutlineFeature::PartEnvelope {
            connections,
            settings,
            id,
            part_ids,
            margin,
            operation,
        } => OutlineFeature::PartEnvelope {
            connections: connections
                .iter()
                .map(|connection| OutlineConnection {
                    id: (renames.identity)(&connection.id),
                    width: connection.width,
                    points: connection
                        .points
                        .iter()
                        .map(|point| mirror_control(point, mapping, renames))
                        .collect(),
                })
                .collect(),
            settings: mirror_settings(settings, mapping, renames),
            id: (renames.identity)(id),
            part_ids: part_ids
                .iter()
                .filter(|part| !part.starts_with(GENERATED_CLOSURE_PREFIX))
                .map(|part| (renames.part)(part))
                .collect(),
            margin: *margin,
            operation: *operation,
        },
    }
}

/// A fixed outline version's snapshot, reflected: features, authored bridges and protected gaps.
fn mirror_snapshot(
    snapshot: &OutlineSnapshot,
    mapping: Pose2,
    renames: &Renames,
    matrix_map: &BTreeMap<String, String>,
) -> OutlineSnapshot {
    OutlineSnapshot {
        features: snapshot
            .features
            .iter()
            .map(|feature| mirror_feature(feature, mapping, renames))
            .collect(),
        settings: mirror_settings(&snapshot.settings, mapping, renames),
        expected_regions: snapshot.expected_regions,
        bridges: snapshot
            .bridges
            .iter()
            .map(|bridge| OutlineBridge {
                id: (renames.identity)(&bridge.id),
                width: bridge.width,
                points: bridge
                    .points
                    .iter()
                    .map(|point| pair::forward(mapping, *point))
                    .collect(),
                part_ids: bridge
                    .part_ids
                    .iter()
                    .map(|part| (renames.part)(part))
                    .collect(),
                matrix_ids: bridge
                    .matrix_ids
                    .iter()
                    .map(|matrix| {
                        matrix_map
                            .get(matrix)
                            .cloned()
                            .unwrap_or_else(|| matrix.clone())
                    })
                    .collect(),
                authored: bridge.authored,
            })
            .collect(),
        protected_gaps: snapshot
            .protected_gaps
            .iter()
            .map(|gap| mirror_gap(gap, mapping, renames))
            .collect(),
    }
}

/// A mirror's reference: the board token substituted, or `to-reference` when none matches.
fn mirror_reference(from: &str, to: &str, reference: &str) -> String {
    substitute_token(reference, from, to).unwrap_or_else(|| format!("{to}-{reference}"))
}

/// Renames a wiring function key or value that names a part or net of the source board
/// (`peripheral/left/TRRS/split-rx`, a net id). The longest whole-segment match wins; a
/// terminal such as `P21` names nothing and is kept.
fn remap_wiring_key(text: &str, renames: &BTreeMap<String, String>) -> String {
    let bounded = |start: usize, end: usize| {
        (start == 0 || text.as_bytes()[start - 1] == b'/')
            && (end == text.len() || text.as_bytes()[end] == b'/')
    };
    let mut best: Option<(usize, usize, &String)> = None;
    for (source, mirror) in renames {
        let mut from = 0;
        while let Some(found) = text[from..].find(source.as_str()) {
            let start = from + found;
            let end = start + source.len();
            if bounded(start, end) && best.is_none_or(|(_, length, _)| source.len() > length) {
                best = Some((start, source.len(), mirror));
                break;
            }
            from = start + 1;
            while !text.is_char_boundary(from) {
                from += 1;
            }
        }
    }
    match best {
        Some((start, length, mirror)) => {
            format!("{}{}{}", &text[..start], mirror, &text[start + length..])
        }
        None => text.to_owned(),
    }
}

fn corresponds(kind: MirroredItemKind, source: &str, mirror: &str) -> MirroredCorrespondence {
    MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind,
        source_id: source.to_owned(),
        mirror_id: mirror.to_owned(),
    }
}

/// An embedded circuit's own members, private nets and component definitions, and where each one
/// mirrors (ticket 28). `modules::circuit` places a circuit's members under `embedded/<id>/` and reads
/// that prefix back (`embed`, `remove_circuit`, the electrical checks), so the board-prefix rule would
/// break it (`embedded/left-haptic/U1` would become `right/embedded/...`). A mirror keeps the prefix and
/// takes its own circuit id: `embedded/left-haptic/U1` becomes `embedded/right-haptic/U1`. The circuit
/// id itself takes the board-prefix rule. A host net a port joins is not in the namespace and keeps the
/// board-prefix rule.
struct CircuitMirror {
    /// Source circuit id to mirror circuit id.
    ids: BTreeMap<String, String>,
    /// Source id of a circuit member or private net to its mirror id.
    owned: BTreeMap<String, String>,
    /// Source id of a circuit component definition to its mirror copy.
    definitions: BTreeMap<String, String>,
}

/// Moves `id` from the namespace `from` to the namespace `to`, or `None` when `id` is outside `from`.
fn reprefix(id: &str, from: &str, to: &str) -> Option<String> {
    id.strip_prefix(from).map(|rest| format!("{to}{rest}"))
}

/// `reference` when the document does not use it yet, otherwise its prefix with the next free number
/// (`right-U1` becomes `right-U2`), as `modules::allocate_reference` does for circuit members.
fn free_reference(reference: &str, used: &mut BTreeSet<String>) -> String {
    if used.insert(reference.to_owned()) {
        return reference.to_owned();
    }
    let prefix = reference.trim_end_matches(|c: char| c.is_ascii_digit());
    let mut index = 1;
    loop {
        let candidate = format!("{prefix}{index}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        index += 1;
    }
}

/// The mirror of each embedded circuit hosted on `from`. A mirror circuit id that is already a circuit
/// on the document, or that another hosted circuit already takes, refuses the record by name, and so
/// does a mirror namespace that already holds a part, net or definition.
fn circuit_mirror(
    doc: &ProjectDoc,
    hosted: &[EmbeddedCircuit],
    from: &str,
    to: &str,
) -> Result<CircuitMirror, String> {
    let mut mirror = CircuitMirror {
        ids: BTreeMap::new(),
        owned: BTreeMap::new(),
        definitions: BTreeMap::new(),
    };
    for circuit in hosted {
        let id = mirror_id(from, to, &circuit.id);
        if doc.embedded_circuits.iter().any(|other| other.id == id)
            || mirror.ids.values().any(|taken| *taken == id)
        {
            return Err(format!(
                "The mirrored embedded circuit {id:?} already exists."
            ));
        }
        let source_prefix = format!("embedded/{}/", circuit.id);
        let prefix = format!("embedded/{id}/");
        if doc.parts.iter().any(|p| p.id.starts_with(&prefix))
            || doc.definitions.iter().any(|d| d.id.starts_with(&prefix))
            || doc.nets.iter().any(|n| n.id.starts_with(&prefix))
        {
            return Err(format!(
                "The circuit namespace {prefix:?} is already in use."
            ));
        }
        for part in &circuit.part_ids {
            let mirrored = reprefix(part, &source_prefix, &prefix).ok_or_else(|| {
                format!(
                    "Embedded circuit {:?} has member {part:?} outside its namespace.",
                    circuit.id
                )
            })?;
            mirror.owned.insert(part.clone(), mirrored);
        }
        for net in &circuit.net_ids {
            if let Some(mirrored) = reprefix(net, &source_prefix, &prefix) {
                mirror.owned.insert(net.clone(), mirrored);
            }
        }
        let (source_definitions, mirror_definitions) = (
            format!("{source_prefix}definition/"),
            format!("{prefix}definition/"),
        );
        for definition in &doc.definitions {
            if let Some(mirrored) =
                reprefix(&definition.id, &source_definitions, &mirror_definitions)
            {
                mirror.definitions.insert(definition.id.clone(), mirrored);
            }
        }
        mirror.ids.insert(circuit.id.clone(), id);
    }
    Ok(mirror)
}

/// Repairs only absent matrix companions on the draft. Generate into a scratch document so
/// existing source parts, authored poses, sides and generator parameters remain untouched.
/// Return the repaired ids: their mirrors keep a durable literal fallback instead of the
/// requested duplicate/tweak mode. A missing switch is a validation error, never a repair.
fn materialise_missing_companions(
    doc: &mut ProjectDoc,
    board_id: &str,
) -> Result<BTreeSet<String>, String> {
    let matrices: Vec<_> = doc
        .matrices
        .iter()
        .filter(|matrix| matrix.board_id.as_deref() == Some(board_id))
        .cloned()
        .collect();
    let mut repaired = BTreeSet::new();
    for matrix in matrices {
        crate::matrix::valid_matrix(&matrix, doc)?;
        let mut missing = BTreeSet::new();
        for row in 0..matrix.rows {
            for column in 0..matrix.columns {
                let cell = matrix
                    .cells
                    .iter()
                    .find(|cell| cell.row == row && cell.column == column);
                if cell.is_some_and(|cell| !cell.enabled || cell.deleted) {
                    continue;
                }
                let key_id = crate::matrix::member_id(&matrix.id, row, column);
                if !doc.parts.iter().any(|part| part.id == key_id) {
                    return Err(format!(
                        "A companion's source switch {key_id:?} is unavailable"
                    ));
                }
                for assembly in cell.into_iter().flat_map(|cell| &cell.assemblies) {
                    let id = crate::matrix::companion_id(&key_id, &assembly.id);
                    if !doc.parts.iter().any(|part| part.id == id) {
                        missing.insert(id);
                    }
                }
            }
        }
        if missing.is_empty() {
            continue;
        }
        let mut generated = doc.clone();
        crate::matrix::set_matrix(&mut generated, &matrix)?;
        doc.parts.extend(
            generated
                .parts
                .into_iter()
                .filter(|part| missing.contains(&part.id)),
        );
        let source_matrix = doc
            .matrices
            .iter_mut()
            .find(|source| source.id == matrix.id)
            .ok_or("The source matrix is unavailable")?;
        let board = doc
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .ok_or("The source board is unavailable")?;
        for id in &missing {
            if !source_matrix.part_ids.contains(id) {
                source_matrix.part_ids.push(id.clone());
            }
            if !board.part_ids.contains(id) {
                board.part_ids.push(id.clone());
            }
        }
        repaired.extend(missing);
    }
    Ok(repaired)
}

fn new_board(
    doc: &mut ProjectDoc,
    request: &Request,
    board_id: &str,
    instance_id: &str,
) -> Result<Vec<String>, String> {
    let mapping = request.mapping;
    let (source, configuration) = source_half(doc, request)?;
    let from = source.board_id.clone();
    let to = board_id.to_owned();
    ensure_board_unclaimed(doc, &from)?;
    if to.trim().is_empty() || doc.boards.iter().any(|board| board.id == to) {
        return Err(format!("Board {to:?} already exists."));
    }
    ensure_instance_fresh(doc, instance_id)?;
    let repaired_companions = materialise_missing_companions(doc, &from)?;
    let source_board = doc
        .boards
        .iter()
        .find(|board| board.id == from)
        .cloned()
        .ok_or("The source board is unavailable")?;
    let hosted: Vec<EmbeddedCircuit> = doc
        .embedded_circuits
        .iter()
        .filter(|circuit| circuit.host_board_id == from)
        .cloned()
        .collect();
    let circuits = circuit_mirror(doc, &hosted, &from, &to)?;

    let map = |id: &str| mirror_id(&from, &to, id);
    // A circuit's member or private net takes its namespace's mirror; everything else the board rule.
    let map_owned = |id: &str| circuits.owned.get(id).cloned().unwrap_or_else(|| map(id));

    // Every new id is minted before anything is written, so a collision refuses the record.
    let mut taken = taken_ids(doc);
    mint(&mut taken, to.clone())?;
    // Generated clearance holes are regenerated from the pair, never copied.
    let source_parts: Vec<&String> = source_board
        .part_ids
        .iter()
        .filter(|id| !id.starts_with(GENERATED_CLOSURE_PREFIX))
        .collect();
    let matrices: Vec<_> = doc
        .matrices
        .iter()
        .filter(|matrix| matrix.board_id.as_deref() == Some(from.as_str()))
        .cloned()
        .collect();
    let mut matrix_map = BTreeMap::new();
    for matrix in &matrices {
        // Matrix ids allow letters, digits, `_` and `-` only, so an id without the board token is
        // joined with `-`, not `/`.
        let mirrored = mint(&mut taken, mirror_reference(&from, &to, &matrix.id))?;
        matrix_map.insert(matrix.id.clone(), mirrored);
    }
    // A matrix places its members itself, under `matrix/<matrix id>/`: follow the mirror matrix.
    let mut part_map = BTreeMap::new();
    for part in &source_parts {
        let owner = matrices
            .iter()
            .find(|matrix| matrix.part_ids.iter().any(|member| member == part.as_str()));
        let mirrored = match owner {
            Some(matrix) => {
                let prefix = format!("matrix/{}/", matrix.id);
                let rest = part.strip_prefix(&prefix).ok_or_else(|| {
                    format!("Matrix member {part:?} is not named under {prefix:?}.")
                })?;
                format!("matrix/{}/{rest}", matrix_map[&matrix.id])
            }
            None => map_owned(part),
        };
        part_map.insert((*part).clone(), mint(&mut taken, mirrored)?);
    }
    let matrix_owned: BTreeSet<&str> = matrices
        .iter()
        .flat_map(|matrix| matrix.part_ids.iter().map(String::as_str))
        .collect();
    // Matrix assemblies are companions by relationship, including imported/plain LEDs without a generator.
    let mut companion_ids = BTreeSet::new();
    for matrix in &matrices {
        for cell in &matrix.cells {
            let key_id = crate::matrix::member_id(&matrix.id, cell.row, cell.column);
            for assembly in &cell.assemblies {
                companion_ids.insert(crate::matrix::companion_id(&key_id, &assembly.id));
            }
        }
    }
    let layouts: Vec<_> = doc
        .layouts
        .iter()
        .filter(|layout| layout.board_id == from)
        .cloned()
        .collect();
    // Mounted modules on the source board take mirror ids by the board-prefix rule, like parts.
    let mut module_map = BTreeMap::new();
    for module in doc.modules.iter().filter(|m| m.host_board_id == from) {
        let mirrored = mint(&mut taken, map(&module.id))?;
        module_map.insert(module.id.clone(), mirrored);
    }
    let mut layout_map = BTreeMap::new();
    for layout in &layouts {
        let mirrored = mint(&mut taken, mirror_reference(&from, &to, &layout.id))?;
        layout_map.insert(layout.id.clone(), mirrored);
    }
    let mut outline_map = BTreeMap::new();
    for outline in &source_board.outline_ids {
        if !doc.outline.iter().any(|feature| feature.id() == outline) {
            return Err(format!("Board outline {outline:?} is unavailable."));
        }
        let mirrored = mint(&mut taken, map(outline))?;
        outline_map.insert(outline.clone(), mirrored);
    }
    let mut net_map = BTreeMap::new();
    for net in &source_board.net_ids {
        if !doc.nets.iter().any(|item| item.id == *net) {
            return Err(format!("Net {net:?} is unavailable."));
        }
        let mirrored = mint(&mut taken, map_owned(net))?;
        net_map.insert(net.clone(), mirrored);
    }
    let board_outline = doc
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == from)
        .cloned();
    let mut version_map = BTreeMap::new();
    if let Some(outline) = &board_outline {
        for version in &outline.versions {
            let mirrored = mint(&mut taken, map(&version.id))?;
            version_map.insert(version.id.clone(), mirrored);
        }
    }
    let electrical = doc.hardware.as_ref().and_then(|hardware| {
        hardware
            .boards
            .iter()
            .find(|entry| entry.board_id == from)
            .cloned()
    });
    let part_set: BTreeSet<&str> = source_parts.iter().map(|id| id.as_str()).collect();
    let mapped_reference = |id: &str| -> String {
        part_map
            .get(id)
            .or_else(|| net_map.get(id))
            .cloned()
            .unwrap_or_else(|| id.to_owned())
    };
    // Outline identities share one namespace: record what the renames mint, check it later.
    let outline_namespace: BTreeSet<String> = crate::outline_edit::persisted_identities(doc)
        .into_iter()
        .collect();
    let minted = RefCell::new(Vec::<(String, String)>::new());
    let record_identity = |id: &str| {
        let mirrored = map(id);
        minted.borrow_mut().push((id.to_owned(), mirrored.clone()));
        mirrored
    };
    let part_reference = |id: &str| mapped_reference(id);
    // Controllers follow the switch rule, like matrix-owned parts: their heading is the plain
    // reflection, so an MCU-anchored outline uses the (-x, y) part frame.
    let controller_parts = controller_parts(doc, &from);
    let full_rule_part = |id: &str| !matrix_owned.contains(id) && !controller_parts.contains(id);
    let renames = Renames {
        identity: &record_identity,
        part: &part_reference,
        full_rule_part: &full_rule_part,
    };

    let mut correspondences = Vec::new();

    // The board. Its part list is set once every part exists.
    doc.boards.push(Board {
        id: to.clone(),
        name: format!("{} (mirror)", source_board.name),
        outline_ids: source_board
            .outline_ids
            .iter()
            .map(|id| outline_map[id].clone())
            .collect(),
        part_ids: vec![],
        net_ids: source_board
            .net_ids
            .iter()
            .map(|id| net_map[id].clone())
            .collect(),
        thickness: source_board.thickness,
        traces: vec![],
        vias: vec![],
    });

    // Stage mapped outlines until matrix placement finishes so `set_matrix` cannot expand authored envelopes.
    let mut mirrored_outlines = Vec::new();
    for outline in &source_board.outline_ids {
        let feature = doc
            .outline
            .iter()
            .find(|feature| feature.id() == outline)
            .cloned()
            .ok_or("Board outline is unavailable")?;
        let mirrored = mirror_feature(&feature, mapping, &renames);
        if mirrored.id() != outline_map[outline] {
            return Err(format!(
                "Board outline {outline:?} was renamed unexpectedly."
            ));
        }
        mirrored_outlines.push(mirrored);
        correspondences.push(corresponds(
            MirroredItemKind::Outline,
            outline,
            &outline_map[outline],
        ));
    }

    // Matrices reflect through the pair map, keeping the source numbering. `set_matrix` places
    // each matrix's members and gives them their ids; their attributes are copied afterwards.
    for matrix in &matrices {
        let mut template = matrix.clone();
        template.id = matrix_map[&matrix.id].clone();
        template.board_id = Some(to.clone());
        template.cells.clear();
        template.part_ids.clear();
        let mut reflected = layout::reflected(matrix, &template, mapping, true)?;
        for cell in &mut reflected.cells {
            if !cell.enabled || cell.deleted {
                continue;
            }
            for assembly in &mut cell.assemblies {
                let key_id = crate::matrix::member_id(&matrix.id, cell.row, cell.column);
                let companion_id = crate::matrix::companion_id(&key_id, &assembly.id);
                let stabiliser = SYMMETRIC_DEFINITIONS.contains(&assembly.definition_id.as_str());
                let fallback = repaired_companions.contains(&companion_id);
                let needs_source_parts = stabiliser
                    || (!fallback
                        && request.companion_mode == MirroredCompanionMode::DuplicatePosition);
                let (key, companion) = if needs_source_parts {
                    let key = doc
                        .parts
                        .iter()
                        .find(|part| part.id == key_id)
                        .ok_or("A companion's source switch is unavailable")?;
                    let companion = doc
                        .parts
                        .iter()
                        .find(|part| part.id == companion_id)
                        .ok_or("A source companion is unavailable")?;
                    (Some(key), Some(companion))
                } else {
                    (None, None)
                };
                let (offset, rotation) =
                    derive::companion_assembly(derive::CompanionAssemblyInput {
                        offset: assembly.offset,
                        rotation: assembly.rotation,
                        key,
                        companion,
                        mapping,
                        mode: request.companion_mode,
                        tweak: request.companion_tweak,
                        stabiliser,
                        fallback,
                    })?;
                assembly.offset = offset;
                assembly.rotation = rotation;
            }
        }
        crate::matrix::set_matrix(doc, &reflected)?;
        correspondences.push(corresponds(
            MirroredItemKind::Matrix,
            &matrix.id,
            &matrix_map[&matrix.id],
        ));
        for member in &matrix.part_ids {
            let source_part = doc
                .parts
                .iter()
                .find(|part| part.id == *member)
                .cloned()
                .ok_or("A matrix member is unavailable")?;
            let mirrored = part_map
                .get(member)
                .ok_or("A matrix member has no mirror id")?
                .clone();
            let placed = doc
                .parts
                .iter_mut()
                .find(|part| part.id == mirrored)
                .ok_or_else(|| format!("The mirror of matrix member {member:?} was not placed."))?;
            placed.keycap = source_part.keycap;
            placed.outline = source_part.outline.clone();
            placed.locked = source_part.locked;
            placed.properties = source_part.properties.clone();
            placed.generator_parameters = source_part.generator_parameters.clone();
        }
    }
    doc.outline.extend(mirrored_outlines);

    // A circuit's component definitions are copied under its mirror namespace. The content is the
    // source's, with only the ids the namespace gives changed (a profile names its own definition).
    let copies: Vec<PartDefinition> = doc
        .definitions
        .iter()
        .filter_map(|definition| {
            let mirrored = circuits.definitions.get(&definition.id)?;
            let mut copy = definition.clone();
            copy.id = mirrored.clone();
            if let Some(profile) = &mut copy.mechanical_profile {
                profile.definition_id = copy.id.clone();
            }
            Some(copy)
        })
        .collect();
    doc.definitions.extend(copies);

    // Parts. Matrix members are already placed; other parts copy and reflect here.
    let mut references: BTreeSet<String> = doc
        .parts
        .iter()
        .map(|part| part.reference.clone())
        .collect();
    // The board's own parts go first, so their mirror references are reserved before a circuit
    // member takes one. A circuit member's reference is allocated on the host board only, so it may
    // equal a host part's reference (the mirror of `U1` is `right-U1` for both `left-U1` and a
    // circuit's `U1`); the member takes the next free reference instead.
    let (own, members): (Vec<&&String>, Vec<&&String>) = source_parts
        .iter()
        .partition(|id| !circuits.owned.contains_key(id.as_str()));
    for id in own.into_iter().chain(members) {
        let mut correspondence = corresponds(MirroredItemKind::Part, id, &part_map[*id]);
        if repaired_companions.contains(id.as_str()) {
            correspondence.companion_mode = Some(MirroredCompanionMode::LiteralMirror);
        } else if companion_ids.contains(id.as_str()) {
            correspondence.companion_tweak = request.companion_tweak;
        }
        correspondences.push(correspondence);
        if matrix_owned.contains(id.as_str()) {
            continue;
        }
        let part = doc
            .parts
            .iter()
            .find(|part| part.id == **id)
            .cloned()
            .ok_or("A board part is unavailable")?;
        let mirrored_reference = mirror_reference(&from, &to, &part.reference);
        let reference = if circuits.owned.contains_key(id.as_str()) {
            free_reference(&mirrored_reference, &mut references)
        } else if references.insert(mirrored_reference.clone()) {
            mirrored_reference
        } else {
            return Err(format!(
                "The mirrored reference {mirrored_reference:?} is already in use."
            ));
        };
        let definition_id = circuits
            .definitions
            .get(&part.definition_id)
            .unwrap_or(&part.definition_id);
        let is_controller = derive::is_controller(&doc.definitions, &part.definition_id);
        doc.parts.push(derive::part(
            &part,
            &part_map[*id],
            definition_id,
            &reference,
            mapping,
            is_controller,
        ));
    }

    // Layouts: independent copies of the source's layouts on the new board.
    for layout in &layouts {
        let matrix_ids = layout
            .matrix_ids
            .iter()
            .map(|id| {
                matrix_map
                    .get(id)
                    .cloned()
                    .ok_or("A layout's matrix is not on its board")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let part_ids = layout
            .part_ids
            .iter()
            .filter(|id| !id.starts_with(GENERATED_CLOSURE_PREFIX))
            .map(|id| {
                part_map
                    .get(id)
                    .cloned()
                    .ok_or_else(|| format!("Layout part {id:?} is not on board {from:?}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        doc.layouts.push(Layout {
            id: layout_map[&layout.id].clone(),
            name: layout.name.clone(),
            board_id: to.clone(),
            matrix_ids,
            part_ids,
            mirror_link: None,
        });
        correspondences.push(corresponds(
            MirroredItemKind::Layout,
            &layout.id,
            &layout_map[&layout.id],
        ));
    }

    // Nets keep their names; a pin on a part of the source board follows the part.
    for net in &source_board.net_ids {
        let source_net = doc
            .nets
            .iter()
            .find(|item| item.id == *net)
            .cloned()
            .ok_or("A board net is unavailable")?;
        let pins = source_net
            .pins
            .iter()
            .filter_map(|pin| {
                part_map.get(&pin.part_id).map(|part_id| Pin {
                    part_id: part_id.clone(),
                    pad_id: pin.pad_id.clone(),
                })
            })
            .collect();
        doc.nets.push(Net {
            id: net_map[net].clone(),
            name: mirror_net_name(&from, &to, &source_net.name),
            pins,
        });
        correspondences.push(corresponds(MirroredItemKind::Net, net, &net_map[net]));
    }

    // Keycap settings keyed by the source board, its matrices and its keys, legends included
    // (a keymap binding is not copied, its legend is). A setting that already names a mirror id
    // is stale data, so it refuses rather than being overwritten.
    if let Some(source_keycaps) = doc.keycaps.clone()
        && let Some(target) = doc.keycaps.as_mut()
    {
        let copy = |existing: bool, key: &str| -> Result<(), String> {
            if existing {
                return Err(format!("A keycap setting already names {key:?}."));
            }
            Ok(())
        };
        if let Some(settings) = source_keycaps.boards.get(&from) {
            copy(target.boards.contains_key(&to), &to)?;
            target.boards.insert(to.clone(), settings.clone());
        }
        for (matrix, settings) in &source_keycaps.matrices {
            if let Some(mirrored) = matrix_map.get(matrix) {
                copy(target.matrices.contains_key(mirrored), mirrored)?;
                target.matrices.insert(mirrored.clone(), settings.clone());
            }
        }
        for (part, settings) in &source_keycaps.keys {
            if let Some(mirrored) = part_map.get(part) {
                copy(target.keys.contains_key(mirrored), mirrored)?;
                target.keys.insert(mirrored.clone(), settings.clone());
            }
        }
    }

    // The electrical configuration of the source board. The mirror keeps the source's matrix
    // numbering, so it keeps the same pins: functions are renamed only where they name a part
    // or net (`peripheral/<part>/...`). Keymap bindings are never reflected, and the protected
    // handoff baseline belongs to the source's reviewed wiring.
    if let Some(entry) = electrical {
        let controller_part_id = entry
            .controller_part_id
            .as_deref()
            .filter(|id| part_set.contains(id))
            .map(mapped_reference);
        let renamed: BTreeMap<String, String> = part_map
            .iter()
            .chain(net_map.iter())
            .map(|(source, mirror)| (source.clone(), mirror.clone()))
            .collect();
        let remap = |map: &BTreeMap<String, String>| -> BTreeMap<String, String> {
            map.iter()
                .map(|(key, value)| {
                    (
                        remap_wiring_key(key, &renamed),
                        remap_wiring_key(value, &renamed),
                    )
                })
                .collect()
        };
        let mirrored = ElectricalBoardConfiguration {
            board_id: to.clone(),
            controller_part_id,
            mode: entry.mode,
            locks: remap(&entry.locks),
            assignments: remap(&entry.assignments),
            key_bindings: BTreeMap::new(),
            jumper_states: entry.jumper_states.clone(),
            protected_handoff: None,
        };
        let hardware = doc
            .hardware
            .as_mut()
            .ok_or("Physical instances are unavailable")?;
        if hardware.boards.iter().any(|entry| entry.board_id == to) {
            return Err(format!(
                "Board {to:?} already has an electrical configuration."
            ));
        }
        hardware.boards.push(mirrored);
        correspondences.push(corresponds(MirroredItemKind::ElectricalBoard, &from, &to));
    }

    // Fixed outline versions: the geometry reflects, and the stored record starts without its
    // generated snapshot or retired identities.
    if let Some(outline) = board_outline {
        if doc.board_outlines.iter().any(|entry| entry.board_id == to) {
            return Err(format!("Board {to:?} already has a board outline."));
        }
        let mut versions = Vec::new();
        for version in &outline.versions {
            let geometry = mirror_snapshot(&version.geometry, mapping, &renames, &matrix_map);
            let provenance = OutlineProvenance {
                revision: version.source.revision,
                version_id: version
                    .source
                    .version_id
                    .as_ref()
                    .and_then(|id| version_map.get(id).cloned()),
            };
            versions.push(OutlineVersion {
                id: version_map[&version.id].clone(),
                name: version.name.clone(),
                source: provenance,
                geometry,
                ownership: version.ownership,
            });
            correspondences.push(corresponds(
                MirroredItemKind::OutlineVersion,
                &version.id,
                &version_map[&version.id],
            ));
        }
        doc.board_outlines.push(BoardOutline {
            board_id: to.clone(),
            active_version_id: outline
                .active_version_id
                .as_ref()
                .and_then(|id| version_map.get(id).cloned()),
            versions,
            generated_last_valid: None,
            retired_version_ids: vec![],
        });
    }

    // Outline identities share one namespace. Every identity the renames made must be new, and
    // two different source identities must not meet at one mirror identity. (A gap kept on the
    // live outline and the same gap protected in a fixed copy share their source identity.)
    let mut sources: BTreeMap<String, String> = BTreeMap::new();
    for (source_identity, identity) in minted.into_inner() {
        let first = sources
            .entry(identity.clone())
            .or_insert_with(|| source_identity.clone());
        if outline_namespace.contains(&identity) || *first != source_identity {
            return Err(format!(
                "The mirrored outline identity {identity:?} is already in use."
            ));
        }
    }

    // The board lists its parts in the source's order.
    let board_parts: Vec<String> = source_parts
        .iter()
        .map(|id| part_map[*id].clone())
        .collect();
    if let Some(board) = doc.boards.iter_mut().find(|board| board.id == to) {
        board.part_ids = board_parts;
    }

    // The second half, then the link: the pair, its shared construction, every linked feature
    // and the generated clearance holes for both halves.
    let mirror = mirror_instance(
        &source,
        instance_id,
        &to,
        false,
        source
            .controller_part_id
            .as_deref()
            .filter(|id| part_set.contains(id))
            .map(mapped_reference),
        mirror_configuration(&configuration, &to, true),
    );
    doc.hardware
        .as_mut()
        .ok_or("Physical instances are unavailable")?
        .instances
        .push(mirror);
    let context = ModuleMirror {
        from: &from,
        to: &to,
        source_instance: request.source_id,
        mirror_instance: instance_id,
        mapping,
        parts: &part_map,
        omitted_parts: &BTreeSet::new(),
        modules: &module_map,
    };
    let mut mirrored_modules = Vec::new();
    for module in doc.modules.iter().filter(|m| m.host_board_id == from) {
        mirrored_modules.push(mirror_module(module, &context)?);
        correspondences.push(corresponds(
            MirroredItemKind::Module,
            &module.id,
            &module_map[&module.id],
        ));
    }
    doc.modules.extend(mirrored_modules);
    // Each embedded circuit becomes a record of the mirror: its members, nets and ports by the mirror ids.
    let mut mirrored_circuits = Vec::new();
    for circuit in &hosted {
        let id = circuits.ids[&circuit.id].clone();
        let outside = |what: &str, item: &str| {
            format!(
                "Embedded circuit {:?} has {what} {item:?} outside board {from:?}.",
                circuit.id
            )
        };
        let part_ids = circuit
            .part_ids
            .iter()
            .map(|part| {
                part_map
                    .get(part)
                    .cloned()
                    .ok_or_else(|| outside("member", part))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let net_ids = circuit
            .net_ids
            .iter()
            .map(|net| net_map.get(net).cloned().ok_or_else(|| outside("net", net)))
            .collect::<Result<Vec<_>, _>>()?;
        let ports = circuit
            .ports
            .iter()
            .map(|(role, net)| {
                net_map
                    .get(net)
                    .cloned()
                    .map(|mirrored| (role.clone(), mirrored))
                    .ok_or_else(|| outside("port net", net))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        mirrored_circuits.push(EmbeddedCircuit {
            id: id.clone(),
            definition_id: circuit.definition_id.clone(),
            host_board_id: to.clone(),
            part_ids,
            net_ids,
            ports,
        });
        correspondences.push(corresponds(
            MirroredItemKind::EmbeddedCircuit,
            &circuit.id,
            &id,
        ));
    }
    doc.embedded_circuits.extend(mirrored_circuits);
    let mut changed: BTreeSet<String> = link_halves(
        doc,
        request,
        instance_id,
        MirroredHalves {
            companion_mode: request.companion_mode,
            source_board_id: from,
            mirror_board_id: to.clone(),
            correspondences: correspondences.clone(),
            exceptions: vec![],
        },
    )?
    .into_iter()
    .collect();
    changed.extend([to, instance_id.to_owned()]);
    changed.extend(circuits.definitions.values().cloned());
    for correspondence in &correspondences {
        changed.insert(correspondence.source_id.clone());
        changed.insert(correspondence.mirror_id.clone());
    }
    Ok(changed.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Vec2;

    #[test]
    fn mirror_ids_substitute_whole_board_tokens_only() {
        let mirror = |id: &str| mirror_id("left", "right", id);
        assert_eq!(mirror("left/U1"), "right/U1");
        assert_eq!(mirror("left-U1"), "right-U1");
        assert_eq!(mirror("left-outline"), "right-outline");
        assert_eq!(mirror("matrix/left-keys"), "matrix/right-keys");
        assert_eq!(
            mirror("matrix/left-keys/r0c0/diode"),
            "matrix/right-keys/r0c0/diode"
        );
        // A token inside a word is not the board.
        assert_eq!(mirror("leftover/U1"), "right/leftover/U1");
        // An id that names no board is prefixed with the mirror board.
        assert_eq!(mirror("outline-1"), "right/outline-1");
    }

    #[test]
    fn a_mirrored_pose_takes_the_pair_heading_rule() {
        let mapping = Pose2 {
            at: Vec2 { x: 10.0, y: 0.0 },
            rotation: 30.0,
        };
        let pose = Pose2 {
            at: Vec2 { x: 0.0, y: 5.0 },
            rotation: 20.0,
        };
        let mirrored = mirror_pose(mapping, pose);
        // The forward_angle (30 - 20 = 10) turned by 180 degrees: 190.
        assert_eq!(mirrored.rotation, 190.0);
        let expected = pair::forward(mapping, pose.at);
        assert_eq!(mirrored.at, expected);
    }

    fn heading_of(mapping: Pose2, rotation: f64) -> f64 {
        mirror_pose(
            mapping,
            Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation,
            },
        )
        .rotation
    }

    fn same_heading(actual: f64, expected: f64) -> bool {
        let turn = (actual - expected).rem_euclid(360.0);
        turn.min(360.0 - turn) < 1e-9
    }

    /// The heading of the mapped direction, read from the mapped vector itself. It does not use
    /// `forward_angle`, so it checks the rule against the geometry rather than against itself.
    fn image_heading(mapping: Pose2, heading: f64) -> f64 {
        let (sin, cos) = heading.to_radians().sin_cos();
        let image = pair::forward_vector(mapping, Vec2 { x: cos, y: sin });
        image.y.atan2(image.x).to_degrees().rem_euclid(360.0)
    }

    #[test]
    fn a_turned_part_faces_outward_on_the_mirror_over_the_full_circle() {
        let axis = Pose2 {
            at: Vec2 { x: 300.0, y: 0.0 },
            rotation: 0.0,
        };
        // A TRRS jack facing left (180) faces right on the mirror, and the reverse. Modulo 180
        // (or `-r`) would leave the left-facing jack at 180.
        assert_eq!(heading_of(axis, 180.0), 0.0);
        assert_eq!(heading_of(axis, 0.0), 180.0);
        // A part facing up or down needs no turn, only the mirrored position.
        assert_eq!(heading_of(axis, 90.0), 90.0);
        assert_eq!(heading_of(axis, 270.0), 270.0);
        // An off-axis part (an LED at 135 degrees) takes 180 - r as well.
        assert_eq!(heading_of(axis, 135.0), 45.0);
    }

    #[test]
    fn every_heading_mirrors_to_the_mapped_direction_under_axis_and_rotated_maps() {
        let maps = [
            Pose2 {
                at: Vec2 { x: 300.0, y: 0.0 },
                rotation: 0.0,
            },
            Pose2 {
                at: Vec2 { x: 310.0, y: 42.0 },
                rotation: 27.0,
            },
        ];
        for map in maps {
            for heading in [0.0, 30.0, 45.0, 90.0, 135.0, 200.0, 270.0, 315.0] {
                let actual = heading_of(map, heading);
                let expected = image_heading(map, heading);
                assert!(
                    same_heading(actual, expected),
                    "map rotation {} heading {heading}: {actual} from the mapped direction {expected}",
                    map.rotation
                );
            }
        }
        // Under the rotated map the rule is 207 - r, so the off-axis LED at 135 goes to 72.
        assert_eq!(heading_of(maps[1], 135.0), 72.0);
    }

    #[test]
    fn the_four_edge_headings_and_an_off_axis_led_are_exact_under_a_rotated_map() {
        // The rule is `rotation + 180 - r`, read independently from the reflection about the
        // axis at 90 + rotation / 2: a direction at r goes to rotation + 180 - r.
        let rotated = Pose2 {
            at: Vec2 { x: 310.0, y: 42.0 },
            rotation: 27.0,
        };
        assert_eq!(heading_of(rotated, 180.0), 27.0);
        assert_eq!(heading_of(rotated, 0.0), 207.0);
        assert_eq!(heading_of(rotated, 90.0), 117.0);
        assert_eq!(heading_of(rotated, 270.0), 297.0);
        assert_eq!(heading_of(rotated, 135.0), 72.0);
    }

    /// The world point of a part-frame point: `at + R(heading) * local`.
    fn world(pose: Pose2, local: Vec2) -> Vec2 {
        let (sin, cos) = pose.rotation.to_radians().sin_cos();
        Vec2 {
            x: pose.at.x + local.x * cos - local.y * sin,
            y: pose.at.y + local.x * sin + local.y * cos,
        }
    }

    #[test]
    fn anchored_points_land_on_the_mapped_world_point_at_every_heading() {
        let maps = [
            Pose2 {
                at: Vec2 { x: 300.0, y: 0.0 },
                rotation: 0.0,
            },
            Pose2 {
                at: Vec2 { x: 310.0, y: 42.0 },
                rotation: 27.0,
            },
        ];
        let local = Vec2 { x: 4.0, y: 2.5 };
        for map in maps {
            for heading in [0.0, 90.0, 180.0, 270.0, 37.0] {
                let source = Pose2 {
                    at: Vec2 { x: 20.0, y: -15.0 },
                    rotation: heading,
                };
                let expected = pair::forward(map, world(source, local));
                // An independent part: the full rule and the (x, -y) frame.
                let part = mirror_pose(map, source);
                let actual = world(part, anchored_local(local, true));
                assert!(
                    (actual.x - expected.x).abs() < 1e-9 && (actual.y - expected.y).abs() < 1e-9,
                    "independent part, map {} heading {heading}: {actual:?} vs {expected:?}",
                    map.rotation
                );
                // A key or companion: the switch rule `rotation - r` and the (-x, y) frame.
                let key = Pose2 {
                    at: part.at,
                    rotation: pair::forward_angle(map, heading),
                };
                let actual = world(key, anchored_local(local, false));
                assert!(
                    (actual.x - expected.x).abs() < 1e-9 && (actual.y - expected.y).abs() < 1e-9,
                    "switch, map {} heading {heading}: {actual:?} vs {expected:?}",
                    map.rotation
                );
            }
        }
    }

    /// The world point of a pad at `local` on a part (a back-side part mirrors its local x, as the
    /// KiCad export does).
    fn pad_world(pose: Pose2, back: bool, local: Vec2) -> Vec2 {
        let x = if back { -local.x } else { local.x };
        world(pose, Vec2 { x, y: local.y })
    }

    /// Ticket 27 verification. The heading rule turns an independent part by `rotation + 180 - r`
    /// and does not flip it, so its pad layout is rotated, not mirrored. A pad lands on the true
    /// mirror of the source pad only when its local `y` is zero (on the heading axis); every other
    /// pad is off by exactly `2 |y|`, with its local `y` reversed. This holds at every 90-degree
    /// heading, so the on-grid case is not certain for a footprint with pads off its heading axis.
    #[test]
    fn a_turned_part_puts_a_pad_on_the_true_mirror_only_when_it_sits_on_the_heading_axis() {
        let maps = [
            Pose2 {
                at: Vec2 { x: 300.0, y: 0.0 },
                rotation: 0.0,
            },
            Pose2 {
                at: Vec2 { x: 310.0, y: 42.0 },
                rotation: 27.0,
            },
        ];
        let offsets = [
            (1.0, 0.0),
            (-2.0, 0.0),
            (0.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (1.0, 0.5),
            (2.0, -3.0),
        ];
        for map in maps {
            for back in [false, true] {
                for heading in [0.0, 90.0, 180.0, 270.0] {
                    let source = Pose2 {
                        at: Vec2 { x: 20.0, y: -15.0 },
                        rotation: heading,
                    };
                    let part = mirror_pose(map, source);
                    for (x, y) in offsets {
                        let local = Vec2 { x, y };
                        let true_mirror = pair::forward(map, pad_world(source, back, local));
                        let placed = pad_world(part, back, local);
                        let error = Vec2 {
                            x: placed.x - true_mirror.x,
                            y: placed.y - true_mirror.y,
                        };
                        // The error vector is the local y reversed, in the mirror part's frame
                        // taken without the half turn: `R(rotation - r) * (0, -2y)`.
                        let (sin, cos) = (map.rotation - heading).to_radians().sin_cos();
                        let expected = Vec2 {
                            x: 2.0 * y * sin,
                            y: -2.0 * y * cos,
                        };
                        let label = format!(
                            "map {} back {back} heading {heading} pad ({x}, {y})",
                            map.rotation
                        );
                        assert!(
                            (error.x - expected.x).abs() < 1e-9
                                && (error.y - expected.y).abs() < 1e-9,
                            "{label}: error {error:?} expected {expected:?}"
                        );
                        let off = error.x.hypot(error.y);
                        assert_eq!(off < 1e-9, y == 0.0, "{label}: off by {off}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_non_finite_mapping_is_refused_before_anything_is_written() {
        let mut doc = ProjectDoc::empty("doc", "doc");
        let before = doc.clone();
        let mapping = Pose2 {
            at: Vec2 {
                x: f64::NAN,
                y: 0.0,
            },
            rotation: 0.0,
        };
        let error = create(
            &mut doc,
            "pair-right",
            "left",
            &MirroredHalfMode::ReuseBoard {
                instance_id: "right".into(),
            },
            mapping,
            MirroredIdRule::BoardPrefix,
            None,
            MirroredCompanionMode::LiteralMirror,
            None,
            None,
        )
        .unwrap_err();
        assert!(error.contains("finite"), "{error}");
        assert_eq!(doc, before);
    }
}
