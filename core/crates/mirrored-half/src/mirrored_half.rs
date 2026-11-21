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

#[doc(hidden)]
pub mod derive;
#[doc(hidden)]
pub mod inverse;
#[doc(hidden)]
pub mod matrix_addition;
mod mirror_build;
#[doc(hidden)]
pub mod reconcile;
use mirror_build::{
    ModuleMirror, Renames, circuit_mirror, corresponds, free_reference, link_halves,
    materialise_missing_companions, mirror_configuration, mirror_feature, mirror_instance,
    mirror_module, mirror_reference, mirror_snapshot, remap_wiring_key, reuse_board,
};
pub use reconcile::{prepare_open, prepare_transaction_with_ledger};

/// The engine edit dispatcher, consulted under the owning session's identity ledger.
pub type ApplyWithLedger = fn(
    &mut ProjectDoc,
    &[crate::model::BoardContours],
    &crate::model::EditOperation,
    &crate::mechanical::edit::MechanicalIdentityLedger,
) -> Result<Vec<String>, String>;

/// The engine dispatch entry points mirrored-half preparation stages its sub-edits
/// through, injected by the engine so this layer never calls back into it directly.
#[derive(Clone, Copy)]
pub struct EditDispatch {
    pub apply_with_ledger: ApplyWithLedger,
    pub apply_unscripted: crate::matrix::deletion::ApplyUnscripted,
}

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
pub fn create(
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
pub fn create_with_physical_setup(
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
pub fn mirror_id(from: &str, to: &str, id: &str) -> String {
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

pub use boardstudio_model::mirrored_half::SYMMETRIC_DEFINITIONS;

pub fn companion_placement_disabled_reason(
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
pub fn heading_findings(doc: &ProjectDoc) -> Vec<Finding> {
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
pub fn module_findings(doc: &ProjectDoc) -> Vec<Finding> {
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
pub fn mirror_pose(mapping: Pose2, pose: Pose2) -> Pose2 {
    derive::pose(mapping, pose, false)
}

/// A test-only window onto `derive::companion_assembly`, the authority for companion placement,
/// so the constraint endpoint rules can be pinned to it. No production caller; always compiled
/// (rather than feature-gated) so this crate never builds in two feature variants.
#[allow(clippy::too_many_arguments)] // Mirrors derive's input record field for field.
pub fn companion_placement(
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
        let mut reflected = layout::reflected(matrix, &template, mapping, true, false)?;
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
mod tests;
