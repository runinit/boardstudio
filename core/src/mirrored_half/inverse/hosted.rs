//! Inverse-map translation of hosted, electrical and keycap requests (ticket 112).
//!
//! The matrix and part families of ticket 110 live in the parent module. This child module owns
//! the rest of the inventory 110 named: mounted modules, embedded circuits, nets, scan and wiring
//! modes and keycap settings, plus `MoveParts`/`RemoveParts` on circuit members (through
//! [`member_is_linked`]). Prepared paired edits call it through the inverse translator before
//! staging changes.
//!
//! Where things live (search the names; line numbers drift):
//! - Dispatch: [`translate`], called by `canonicalize`'s arm for these operations. It either
//!   rewrites `canonical.operation`, records a planned addition in [`Plan`], or sets the ledger
//!   entry of a family that is shared or deliberately unchanged.
//! - Modules: `set_module` (an existing linked module) and `add_module` (new on a mirror board);
//!   `source_connection` maps the connector, bus and upstream references.
//! - Circuits: `embed_circuit` plans member, definition, net and reference maps for the forward
//!   AddCircuit transition; `remove_circuit` and [`member_is_linked`] keep the namespace and
//!   member map honest.
//! - Nets: `set_net` applies the mirror edit as a pin delta on the source net.
//!
//! Rules this layer keeps (ADR-0010, ticket 112):
//! - A target that is not a linked mirror (unpaired, a source, a Local exception) executes as
//!   written; an Omitted target is refused, as in 110.
//! - Module and circuit poses use the exact inverse of the independent-part rule; every other
//!   payload value is shared with the source and is carried unchanged. Ids and references map.
//! - Shared definitions (module definitions, host connector definitions, circuit definitions)
//!   execute once and are never inverse-transformed.
//! - A value the mirror shows unchanged keeps the source's own value, so an unrelated edit never
//!   rewrites a reference the source spells differently.
//! - A reference the source half cannot follow (a mirror-only connector, upstream module or host
//!   net) refuses the request instead of being dropped.

#[cfg(test)]
#[path = "hosted_tests.rs"]
mod tests;

use super::{Canonical, Coverage, Link, Links, PairView, PartAddition, Target};
use crate::mechanical::pair;
use crate::model::{
    EditOperation, MirroredExceptionState, MirroredItemKind, PartDefinition, Pose2, Side, Vec2,
};
use crate::modules::{ModuleConnection, ModuleDefinition, MountedModule};
use std::collections::{BTreeMap, BTreeSet};

const SHARED_DEFINITION: &str = "a module definition is shared by both halves: the mirror and the source name the same \
     definition and its geometry is local to the module";

/// A mounted module added on a mirror board, planned as a source/mirror pair for the forward
/// AddModule transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::mirrored_half) struct ModuleAddition {
    pub(in crate::mirrored_half) source_id: String,
    pub(in crate::mirrored_half) mirror_id: String,
}

/// An embedded circuit added on a mirror board, planned with the maps the forward AddCircuit
/// transition selects. Every map is keyed by the source identity.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::mirrored_half) struct CircuitAddition {
    pub(in crate::mirrored_half) source_id: String,
    pub(in crate::mirrored_half) mirror_id: String,
    /// Source member id to mirror member id, `embedded/<id>/<member>` on both sides.
    pub(in crate::mirrored_half) member_ids: BTreeMap<String, String>,
    /// Source member id to the reference the mirror member takes on its board.
    pub(in crate::mirrored_half) member_references: BTreeMap<String, String>,
    /// Source circuit definition id to mirror definition id.
    pub(in crate::mirrored_half) definition_ids: BTreeMap<String, String>,
    /// Source circuit net id (a private net or a joined host net) to its mirror id.
    pub(in crate::mirrored_half) net_ids: BTreeMap<String, String>,
    /// Source member id to the world pose the source member takes after the source embed, so the
    /// forward rule lands the mirror member at its typed pose.
    pub(in crate::mirrored_half) member_poses: BTreeMap<String, Pose2>,
}

/// The hosted additions one request plans. A host connector placed atomically with a module is a
/// [`PartAddition`] in `Canonical::additions`.
#[derive(Debug, Default)]
pub(in crate::mirrored_half) struct Plan {
    pub(in crate::mirrored_half) modules: Vec<ModuleAddition>,
    pub(in crate::mirrored_half) circuits: Vec<CircuitAddition>,
}

/// Translates one hosted, electrical or keycap request into `canonical`.
pub(super) fn translate(
    links: &Links<'_>,
    operation: &EditOperation,
    canonical: &mut Canonical,
) -> Result<(), String> {
    use EditOperation as Op;
    match operation {
        Op::SetModuleDefinition { .. } => {
            canonical.coverage = Coverage::DeliberatelyUnchanged(SHARED_DEFINITION);
        }
        Op::SetMountedModule {
            instance,
            definition,
            host_connector_definition,
        } => set_module(
            links,
            instance,
            definition.as_deref(),
            host_connector_definition.as_deref(),
            canonical,
        )?,
        Op::RemoveMountedModule { id } => {
            if let Some(source) = counterpart_or_written(links, MirroredItemKind::Module, id)? {
                canonical.operation = Op::RemoveMountedModule { id: source };
            }
        }
        Op::EmbedModuleCircuit {
            id,
            definition,
            host_board_id,
            pose,
            side,
            joins,
        } => embed_circuit(
            links,
            id,
            definition,
            host_board_id,
            *pose,
            side,
            joins,
            canonical,
        )?,
        Op::RemoveEmbeddedCircuit { id } => remove_circuit(links, id, canonical)?,
        Op::SetInputScanMode { part_id, mode } => {
            if let Some(link) = links.linked_part(part_id)? {
                canonical.operation = Op::SetInputScanMode {
                    part_id: link.correspondence.source_id.clone(),
                    mode: *mode,
                };
            }
        }
        Op::SetWiringMode { board_id, mode } => {
            if let Some(source) =
                counterpart_or_written(links, MirroredItemKind::ElectricalBoard, board_id)?
            {
                canonical.operation = Op::SetWiringMode {
                    board_id: source,
                    mode: *mode,
                };
            }
        }
        // Keycap settings are keyed by board id and have no exception kind of their own; the
        // ElectricalBoard exception governs a board's wiring configuration only (`SetWiringMode`),
        // so a Local electrical board does not move the keycap settings off the pair.
        Op::SetKeycapBoard { board_id, change } => {
            if let Some(source) = source_board(links, board_id)? {
                canonical.operation = Op::SetKeycapBoard {
                    board_id: source,
                    change: change.clone(),
                };
            }
        }
        Op::SetMatrixKeycaps { matrix_id, change } => {
            canonical.operation = Op::SetMatrixKeycaps {
                matrix_id: links.matrix_id(matrix_id)?,
                change: change.clone(),
            };
        }
        Op::SetKeycapKey { key_id, change } => {
            if let Some(link) = links.linked_part(key_id)? {
                canonical.operation = Op::SetKeycapKey {
                    key_id: link.correspondence.source_id.clone(),
                    change: change.clone(),
                };
            }
        }
        // Every other family has its own translation in the parent module.
        Op::MoveParts { .. }
        | Op::AddPart { .. }
        | Op::AddMirrorOnlyPart { .. }
        | Op::RemoveParts { .. }
        | Op::RemoveMatrix { .. }
        | Op::RemoveMatrixCells { .. }
        | Op::RemoveMatrixRow { .. }
        | Op::RemoveMatrixColumn { .. }
        | Op::SetMatrixCellsEnabled { .. }
        | Op::RemoveSelectedMatrixCells { .. }
        | Op::SetMatrixSplay { .. }
        | Op::SetMatrixPlacement { .. }
        | Op::AddAnchoredMatrix { .. }
        | Op::SetMatrixAnchor { .. }
        | Op::DetachMatrixAnchor { .. }
        | Op::DetachMatrixAnchorConfirmed { .. }
        | Op::SetMatrix { .. }
        | Op::EditKeymap { .. }
        | Op::SetKeyBinding { .. }
        | Op::ApplyMechanicalEdit { .. }
        | Op::SetKeycapSize { .. }
        | Op::ApplyOutlineEdit { .. }
        | Op::SetLayout { .. }
        | Op::SetSplitGap { .. }
        | Op::SetMirroredPart { .. }
        | Op::CreateMirroredPair { .. }
        | Op::SetConstraint { .. }
        | Op::RemoveConstraint { .. }
        | Op::ConvertOffsetConstraint { .. }
        | Op::SetCase { .. }
        | Op::SetHalfException { .. }
        | Op::SetCompanionPlacement { .. }
        | Op::UnlinkHalves { .. }
        | Op::CreateMirroredHalf { .. }
        | Op::ReplaceDocument { .. } => {
            return Err("this family is not translated by the hosted inverse".into());
        }
    }
    Ok(())
}

// ---- shared helpers -----------------------------------------------------------------------

/// The source id of a linked mirror `id`, or `None` when the request executes as written.
fn counterpart_or_written(
    links: &Links<'_>,
    kind: MirroredItemKind,
    id: &str,
) -> Result<Option<String>, String> {
    match links.target(kind, id)? {
        Target::NotMirror | Target::Local => Ok(None),
        Target::Omitted => Err(Links::omitted(id)),
        Target::Linked(link) => Ok(Some(link.correspondence.source_id.clone())),
    }
}

/// The source id the source half spells for a mirror reference. A reference the source cannot
/// follow (not a corresponded item, or a Local or Omitted one) refuses the request.
fn followed(
    links: &Links<'_>,
    kind: MirroredItemKind,
    id: &str,
    owner: &str,
    what: &str,
) -> Result<String, String> {
    match links.target(kind, id)? {
        Target::Linked(link) => Ok(link.correspondence.source_id.clone()),
        Target::NotMirror | Target::Local | Target::Omitted => Err(format!(
            "{owner:?} names the {what} {id:?}, which has no source on the other half; the source \
             edit cannot follow it"
        )),
    }
}

/// The pose the source takes so that the independent-part rule lands the mirror on `at` and
/// `rotation`: the exact inverse of `derive::pose` (each heading rule is its own inverse).
/// Controllers keep the reflected heading without the half turn, like the forward rule.
fn source_pose(mapping: Pose2, at: Vec2, rotation: f64, is_controller: bool) -> Pose2 {
    Pose2 {
        at: pair::inverse(mapping, at),
        rotation: pair::mirrored_part_heading(is_controller, mapping, rotation),
    }
}

fn unique_instance(links: &Links<'_>, board_id: &str) -> Result<String, String> {
    let hardware = links
        .document
        .hardware
        .as_ref()
        .ok_or_else(|| "Physical instances are unavailable".to_owned())?;
    let mut matches = hardware
        .instances
        .iter()
        .filter(|instance| instance.board_id == board_id);
    let first = matches
        .next()
        .ok_or_else(|| format!("Board {board_id:?} has no physical instance"))?;
    if matches.next().is_some() {
        return Err(format!(
            "Board {board_id:?} has ambiguous physical instances"
        ));
    }
    Ok(first.id.clone())
}

/// The pair whose mirror half is `board_id`, or `None` when no pair relates it as a mirror (a
/// reused board has the same board on both sides and is never a mirror here).
fn pair_for_mirror_board<'a, 'b>(
    links: &'b Links<'a>,
    board_id: &str,
) -> Result<Option<&'b PairView<'a>>, String> {
    let mut found = links.pairs.iter().filter(|pair| {
        pair.halves.mirror_board_id == board_id && pair.halves.source_board_id != board_id
    });
    let first = found.next();
    if found.next().is_some() {
        return Err(format!(
            "{board_id:?} is the mirror board of more than one pair"
        ));
    }
    Ok(first)
}

fn source_board(links: &Links<'_>, board_id: &str) -> Result<Option<String>, String> {
    Ok(pair_for_mirror_board(links, board_id)?.map(|pair| pair.halves.source_board_id.clone()))
}

// ---- mounted modules ------------------------------------------------------------------------

/// The source half's spelling of a mirror module's connection. A value the mirror shows
/// unchanged keeps the source's own value.
fn source_connection(
    links: &Links<'_>,
    pair: &PairView<'_>,
    module_id: &str,
    payload: &ModuleConnection,
    mirror: Option<&ModuleConnection>,
    source: Option<&ModuleConnection>,
) -> Result<ModuleConnection, String> {
    let (from, to) = (
        pair.halves.source_board_id.as_str(),
        pair.halves.mirror_board_id.as_str(),
    );
    let mut out = payload.clone();
    // An unchanged mirror value keeps the source's own spelling of it.
    let unchanged = |same: fn(&ModuleConnection, &ModuleConnection) -> bool| match (mirror, source)
    {
        (Some(mirror), Some(source)) if same(mirror, payload) => Some(source),
        _ => None,
    };

    let connector = payload.host_connector_part_id.as_str();
    out.host_connector_part_id = if connector.is_empty() {
        String::new()
    } else if let Some(source) =
        unchanged(|a, b| a.host_connector_part_id == b.host_connector_part_id)
    {
        source.host_connector_part_id.clone()
    } else {
        followed(
            links,
            MirroredItemKind::Part,
            connector,
            module_id,
            "host connector",
        )?
    };

    out.bus_id = if payload.bus_id.is_empty() {
        // No bus yet (a module attached before a bus exists) stays unassigned on the source.
        String::new()
    } else if let Some(source) = unchanged(|a, b| a.bus_id == b.bus_id) {
        source.bus_id.clone()
    } else {
        source_bus(from, to, &payload.bus_id).ok_or_else(|| {
            format!(
                "The bus {:?} of module {module_id:?} is not the mirror of any bus id on board \
                 {from:?}",
                payload.bus_id
            )
        })?
    };

    out.upstream_module_id = match payload.upstream_module_id.as_deref() {
        None => None,
        Some(upstream) => Some(
            match unchanged(|a, b| a.upstream_module_id == b.upstream_module_id)
                .and_then(|source| source.upstream_module_id.clone())
            {
                Some(source) => source,
                None => followed(
                    links,
                    MirroredItemKind::Module,
                    upstream,
                    module_id,
                    "upstream module",
                )?,
            },
        ),
    };
    Ok(out)
}

/// The id `source_bus` spells on the source board whose board-prefix mirror is `bus`, or `None`
/// when no source spelling maps there.
fn source_bus(from: &str, to: &str, bus: &str) -> Option<String> {
    // The pair's own spelling first (`right/bus` is `left/bus`), then an unprefixed id that the
    // forward rule prefixed.
    let mut candidates = Vec::new();
    if let Some(swapped) = super::super::substitute_token(bus, to, from) {
        candidates.push(swapped);
    }
    if let Some(rest) = bus.strip_prefix(&format!("{to}/")) {
        candidates.push(rest.to_owned());
    }
    candidates
        .into_iter()
        .find(|candidate| super::super::mirror_id(from, to, candidate) == bus)
}

fn source_instance_pin(
    links: &Links<'_>,
    pair: &PairView<'_>,
    module_id: &str,
    pinned: Option<&String>,
) -> Result<Option<String>, String> {
    let Some(pinned) = pinned else {
        return Ok(None);
    };
    let mirror_instance = unique_instance(links, &pair.halves.mirror_board_id)?;
    if *pinned != mirror_instance {
        return Err(format!(
            "Module {module_id:?} is pinned to physical instance {pinned:?}, which is not the \
             mirror's {mirror_instance:?}; its source has no instance to be pinned to."
        ));
    }
    Ok(Some(unique_instance(links, &pair.halves.source_board_id)?))
}

/// The source module that makes `payload` (a mirror module) what the mirror shows.
fn source_module(
    links: &Links<'_>,
    pair: &PairView<'_>,
    source_id: &str,
    payload: &MountedModule,
    current_mirror: Option<&MountedModule>,
    current_source: Option<&MountedModule>,
) -> Result<MountedModule, String> {
    if payload.host_board_id != pair.halves.mirror_board_id {
        return Err(format!(
            "The linked mirror module {:?} stays on board {:?}; hosting it elsewhere is not \
             reflected from its source",
            payload.id, pair.halves.mirror_board_id
        ));
    }
    let mut source = payload.clone();
    source.id = source_id.to_owned();
    source.host_board_id = pair.halves.source_board_id.clone();
    source.host_instance_id =
        source_instance_pin(links, pair, &payload.id, payload.host_instance_id.as_ref())?;
    // Mounted modules take the full heading rule, like every non-controller independent part.
    let pose = source_pose(pair.mapping, payload.at, payload.rotation, false);
    source.at = pose.at;
    source.rotation = pose.rotation;
    source.connection = payload
        .connection
        .as_ref()
        .map(|connection| {
            source_connection(
                links,
                pair,
                &payload.id,
                connection,
                current_mirror.and_then(|module| module.connection.as_ref()),
                current_source.and_then(|module| module.connection.as_ref()),
            )
        })
        .transpose()?;
    // A mirror that shows no connection while its source has one (its connector is Omitted, so
    // the forward rule cleared it) and still shows none has not edited the connection.
    if let (Some(mirror), Some(current)) = (current_mirror, current_source)
        && payload.connection.is_none()
        && mirror.connection.is_none()
    {
        source.connection = current.connection.clone();
    }
    // The pose of an unchanged mirror is the source's own, bit for bit.
    if let (Some(mirror), Some(current)) = (current_mirror, current_source)
        && mirror.at == payload.at
        && mirror.rotation == payload.rotation
    {
        source.at = current.at;
        source.rotation = current.rotation;
    }
    Ok(source)
}

fn set_module(
    links: &Links<'_>,
    instance: &MountedModule,
    definition: Option<&ModuleDefinition>,
    connector: Option<&PartDefinition>,
    canonical: &mut Canonical,
) -> Result<(), String> {
    let current_mirror = links
        .document
        .modules
        .iter()
        .find(|module| module.id == instance.id);
    let link = match links.target(MirroredItemKind::Module, &instance.id)? {
        Target::Local => return Ok(()),
        Target::Omitted => return Err(Links::omitted(&instance.id)),
        Target::NotMirror if current_mirror.is_some() => return Ok(()),
        Target::NotMirror => {
            return add_module(links, instance, definition, connector, canonical);
        }
        Target::Linked(link) => link,
    };
    let current_mirror = current_mirror
        .ok_or_else(|| format!("The linked mirror module {:?} is unavailable", instance.id))?;
    let source_id = link.correspondence.source_id.as_str();
    let current_source = links
        .document
        .modules
        .iter()
        .find(|module| module.id == source_id)
        .ok_or_else(|| format!("The source module {source_id:?} is unavailable"))?;
    let source = source_module(
        links,
        link.pair,
        source_id,
        instance,
        Some(current_mirror),
        Some(current_source),
    )?;
    canonical.operation = EditOperation::SetMountedModule {
        instance: Box::new(source),
        definition: definition.cloned().map(Box::new),
        host_connector_definition: connector.cloned().map(Box::new),
    };
    Ok(())
}

/// The reference `modules::host_connector` allocates for a generated host connector.
fn host_connector_reference(links: &Links<'_>, board_id: &str) -> String {
    let used: BTreeSet<&str> = links
        .document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .into_iter()
        .flat_map(|board| board.part_ids.iter())
        .filter_map(|id| {
            links
                .document
                .parts
                .iter()
                .find(|part| &part.id == id)
                .map(|part| part.reference.as_str())
        })
        .collect();
    (1..=999)
        .map(|number| format!("J_VIK{number}"))
        .find(|reference| !used.contains(reference.as_str()))
        .unwrap_or_else(|| "J_VIK_AUTO".into())
}

/// A module that is new on a mirror board becomes a source addition, and a host connector it
/// creates is planned as an independent part pair.
fn add_module(
    links: &Links<'_>,
    instance: &MountedModule,
    definition: Option<&ModuleDefinition>,
    connector: Option<&PartDefinition>,
    canonical: &mut Canonical,
) -> Result<(), String> {
    let Some(pair) = pair_for_mirror_board(links, &instance.host_board_id)? else {
        return Ok(());
    };
    let named = |id: &str| {
        links.pairs.iter().any(|pair| {
            pair.halves.correspondences.iter().any(|correspondence| {
                correspondence.kind == MirroredItemKind::Module
                    && (correspondence.source_id == id || correspondence.mirror_id == id)
            })
        })
    };
    if named(&instance.id) {
        return Err(format!(
            "The module {:?} is already named by a correspondence",
            instance.id
        ));
    }
    let candidate = super::super::mirror_id(
        &pair.halves.mirror_board_id,
        &pair.halves.source_board_id,
        &instance.id,
    );
    let generated = |module: &str| format!("{module}/vik-host-connector");
    let part_taken = |id: &str| {
        links.document.parts.iter().any(|part| part.id == id)
            || links.pairs.iter().any(|pair| {
                pair.halves.correspondences.iter().any(|correspondence| {
                    correspondence.source_id == id || correspondence.mirror_id == id
                })
            })
    };
    let mut source_id = candidate.clone();
    let mut index = 2;
    while source_id == instance.id
        || links
            .document
            .modules
            .iter()
            .any(|module| module.id == source_id)
        || named(&source_id)
        || part_taken(&generated(&source_id))
    {
        source_id = format!("{candidate}-{index}");
        index += 1;
    }
    // The connector the operation creates takes `<module id>/vik-host-connector`: a mirror
    // module's connector is planned as the mirror of the source module's own. It does not exist
    // yet, so it is staged as empty while the rest of the connection is translated.
    let creates_connector = connector.is_some()
        && instance.connection.as_ref().is_some_and(|connection| {
            connection.host_connector_part_id.is_empty()
                || connection.host_connector_part_id == generated(&instance.id)
        });
    let names_generated = creates_connector
        && instance
            .connection
            .as_ref()
            .is_some_and(|connection| !connection.host_connector_part_id.is_empty());
    let mut staged = instance.clone();
    if names_generated && let Some(connection) = staged.connection.as_mut() {
        connection.host_connector_part_id = String::new();
    }
    let mut source = source_module(links, pair, &source_id, &staged, None, None)?;
    let mut planned = None;
    if creates_connector {
        let mirror_part = generated(&instance.id);
        let source_part = generated(&source_id);
        if links
            .document
            .parts
            .iter()
            .any(|part| part.id == mirror_part)
        {
            return Err(format!(
                "The generated host connector {mirror_part:?} already exists"
            ));
        }
        if names_generated && let Some(connection) = source.connection.as_mut() {
            connection.host_connector_part_id = source_part.clone();
        }
        planned = Some(PartAddition {
            source_id: source_part,
            mirror_id: mirror_part,
            mirror_definition_id: connector.map(|item| item.id.clone()).unwrap_or_default(),
            mirror_reference: host_connector_reference(links, &pair.halves.mirror_board_id),
            mirror_layout_id: None,
        });
    }
    canonical.hosted.modules.push(ModuleAddition {
        source_id: source.id.clone(),
        mirror_id: instance.id.clone(),
    });
    canonical.operation = EditOperation::SetMountedModule {
        instance: Box::new(source),
        definition: definition.cloned().map(Box::new),
        host_connector_definition: connector.cloned().map(Box::new),
    };
    canonical.additions.extend(planned);
    Ok(())
}

// ---- embedded circuits ----------------------------------------------------------------------

#[allow(clippy::too_many_arguments)] // One field per payload value of the operation.
fn embed_circuit(
    links: &Links<'_>,
    id: &str,
    definition: &ModuleDefinition,
    host_board_id: &str,
    pose: Pose2,
    side: &Side,
    joins: &BTreeMap<String, String>,
    canonical: &mut Canonical,
) -> Result<(), String> {
    // Hosted additions need a plan from either authoring half. A board may participate in
    // several pairs, so select one unambiguous pair before deriving any counterpart identity.
    let mut matches = links.pairs.iter().filter(|pair| {
        (pair.halves.mirror_board_id == host_board_id
            && pair.halves.source_board_id != host_board_id)
            || (pair.halves.source_board_id == host_board_id
                && pair.halves.mirror_board_id != host_board_id)
    });
    let Some(pair) = matches.next() else {
        return Ok(());
    };
    if matches.next().is_some() {
        return Err(format!(
            "{host_board_id:?} belongs to more than one native mirrored pair"
        ));
    }
    let mirror_authored = pair.halves.mirror_board_id == host_board_id;
    // An existing identity is the operation's own refusal; a source without a circuit too.
    let Some(circuit) = definition.circuit.as_ref() else {
        return Ok(());
    };
    if links
        .document
        .embedded_circuits
        .iter()
        .any(|existing| existing.id == id)
    {
        return Ok(());
    }
    let named = |candidate: &str| {
        links
            .document
            .embedded_circuits
            .iter()
            .any(|existing| existing.id == candidate)
            || links.pairs.iter().any(|pair| {
                pair.halves.correspondences.iter().any(|correspondence| {
                    correspondence.kind == MirroredItemKind::EmbeddedCircuit
                        && (correspondence.source_id == candidate
                            || correspondence.mirror_id == candidate)
                })
            })
    };
    let namespace_used = |circuit_id: &str| {
        let prefix = format!("embedded/{circuit_id}/");
        links
            .document
            .parts
            .iter()
            .any(|part| part.id.starts_with(&prefix))
            || links
                .document
                .definitions
                .iter()
                .any(|item| item.id.starts_with(&prefix))
            || links
                .document
                .nets
                .iter()
                .any(|net| net.id.starts_with(&prefix))
            || links.pairs.iter().any(|pair| {
                pair.halves.correspondences.iter().any(|correspondence| {
                    correspondence.source_id.starts_with(&prefix)
                        || correspondence.mirror_id.starts_with(&prefix)
                })
            })
    };
    if named(id) {
        return Err(format!(
            "The circuit {id:?} is already named by a correspondence"
        ));
    }
    if namespace_used(id) {
        return Err(format!(
            "The circuit namespace \"embedded/{id}/\" is already in use"
        ));
    }
    let (source_id, mirror_id) = if mirror_authored {
        let candidate = super::super::mirror_id(
            &pair.halves.mirror_board_id,
            &pair.halves.source_board_id,
            id,
        );
        let mut source_id = candidate.clone();
        let mut index = 2;
        while source_id == id || named(&source_id) || namespace_used(&source_id) {
            source_id = format!("{candidate}-{index}");
            index += 1;
        }
        (source_id, id.to_owned())
    } else {
        let candidate = super::super::mirror_id(
            &pair.halves.source_board_id,
            &pair.halves.mirror_board_id,
            id,
        );
        let mut mirror_id = candidate.clone();
        let mut index = 2;
        while mirror_id == id || named(&mirror_id) || namespace_used(&mirror_id) {
            mirror_id = format!("{candidate}-{index}");
            index += 1;
        }
        (id.to_owned(), mirror_id)
    };

    // Reuse an existing joined host-Net correspondence. Mirror-authored joins follow it;
    // source-authored joins find the unique mirror spelling in this same pair.
    let mut source_joins = BTreeMap::new();
    let mut joined_pairs: BTreeMap<String, (String, String)> = BTreeMap::new();
    for (port, authored_net) in joins {
        let (source_net, mirror_net) = if mirror_authored {
            let source_net = followed(
                links,
                MirroredItemKind::Net,
                authored_net,
                &format!("circuit {id}"),
                "joined host net",
            )?;
            (source_net, authored_net.clone())
        } else {
            let mut correspondences = pair.halves.correspondences.iter().filter(|item| {
                item.kind == MirroredItemKind::Net && item.source_id == *authored_net
            });
            let Some(correspondence) = correspondences.next() else {
                return Err(format!(
                    "circuit {id:?} names joined source net {authored_net:?}, which has no mirror"
                ));
            };
            if correspondences.next().is_some() {
                return Err(format!(
                    "source net {authored_net:?} has more than one mirror correspondence"
                ));
            }
            if let Some(exception) = pair.halves.overlay_of(correspondence) {
                let state = match exception.state {
                    crate::model::MirroredExceptionState::Local => "local",
                    crate::model::MirroredExceptionState::Omitted => "omitted",
                };
                return Err(format!(
                    "source net {authored_net:?} has a {state} mirror exception and cannot be joined"
                ));
            }
            (authored_net.clone(), correspondence.mirror_id.clone())
        };
        if let Some(local) = circuit.ports.get(port) {
            joined_pairs.insert(local.clone(), (source_net.clone(), mirror_net));
        }
        source_joins.insert(port.clone(), source_net);
    }

    // The circuit record itself is not a part, so the embed pose takes the full heading rule.
    let source_embed_pose = if mirror_authored {
        source_pose(pair.mapping, pose.at, pose.rotation, false)
    } else {
        pose
    };
    // Stage the source embed through its ordinary board-local allocator, then reserve those
    // exact references before selecting mirror references for the document-wide validator.
    let source_operation = EditOperation::EmbedModuleCircuit {
        id: source_id.clone(),
        definition: definition.clone(),
        host_board_id: pair.halves.source_board_id.clone(),
        pose: source_embed_pose,
        side: side.clone(),
        joins: source_joins,
    };
    let mut staged_source = links.document.clone();
    crate::apply(&mut staged_source, &[], &source_operation)?;

    let (source_prefix, mirror_prefix) = (
        format!("embedded/{source_id}/"),
        format!("embedded/{mirror_id}/"),
    );
    let mut member_ids = BTreeMap::new();
    let mut member_references = BTreeMap::new();
    let mut used_references: BTreeSet<String> = links
        .document
        .parts
        .iter()
        .map(|part| part.reference.clone())
        .collect();
    for member in &circuit.parts {
        let source_member = format!("{source_prefix}{}", member.id);
        let staged_reference = staged_source
            .parts
            .iter()
            .find(|part| part.id == source_member)
            .ok_or_else(|| format!("the source circuit is missing member {source_member:?}"))?
            .reference
            .clone();
        used_references.insert(staged_reference);
    }
    for member in &circuit.parts {
        let source_member = format!("{source_prefix}{}", member.id);
        member_references.insert(
            source_member.clone(),
            crate::modules::allocate_reference(&member.reference, &mut used_references),
        );
        member_ids.insert(source_member, format!("{mirror_prefix}{}", member.id));
    }
    let definition_ids = circuit
        .definitions
        .iter()
        .map(|item| {
            (
                format!("{source_prefix}definition/{}", item.id),
                format!("{mirror_prefix}definition/{}", item.id),
            )
        })
        .collect();
    let net_ids = circuit
        .nets
        .iter()
        .map(|net| match joined_pairs.get(&net.id) {
            Some((source, mirror)) => (source.clone(), mirror.clone()),
            None => (
                format!("{source_prefix}{}", net.id),
                format!("{mirror_prefix}{}", net.id),
            ),
        })
        .collect();

    // The forward rule reflects each member as an independent part, so a member lands as typed
    // exactly when the source member sits at the inverse image of its typed world pose. Typed
    // poses are what `modules::embed` gives the members on the mirror board: the back side
    // negates the member's x and heading (and flips its side, which the source embed repeats).
    // Controller members keep the reflected heading, matching the forward part rule.
    let member_poses = if mirror_authored {
        let back = *side == Side::Back;
        let (sin, cos) = pose.rotation.to_radians().sin_cos();
        circuit
            .parts
            .iter()
            .map(|member| {
                let x = if back {
                    -member.pose.at.x
                } else {
                    member.pose.at.x
                };
                let typed_at = Vec2 {
                    x: pose.at.x + x * cos - member.pose.at.y * sin,
                    y: pose.at.y + x * sin + member.pose.at.y * cos,
                };
                let typed_rotation = pose.rotation
                    + if back {
                        -member.pose.rotation
                    } else {
                        member.pose.rotation
                    };
                let is_controller = circuit.definitions.iter().any(|definition| {
                    definition.id == member.definition_id
                        && definition.kind == crate::model::PartKind::Controller
                });
                (
                    format!("{source_prefix}{}", member.id),
                    source_pose(pair.mapping, typed_at, typed_rotation, is_controller),
                )
            })
            .collect()
    } else {
        BTreeMap::new()
    };

    if mirror_authored {
        canonical.operation = source_operation;
    }
    canonical.hosted.circuits.push(CircuitAddition {
        source_id,
        mirror_id,
        member_ids,
        member_references,
        definition_ids,
        net_ids,
        member_poses,
    });
    Ok(())
}

fn remove_circuit(links: &Links<'_>, id: &str, canonical: &mut Canonical) -> Result<(), String> {
    let Some(source) = counterpart_or_written(links, MirroredItemKind::EmbeddedCircuit, id)? else {
        return Ok(());
    };
    // The members' lock is the mirror's too: removing the source circuit removes the mirror's.
    let locked = links
        .document
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.id == id)
        .into_iter()
        .flat_map(|circuit| circuit.part_ids.iter())
        .any(|member| {
            links
                .document
                .parts
                .iter()
                .any(|part| &part.id == member && part.locked == Some(true))
        });
    if locked {
        return Err("Unlock circuit members before removing the circuit".into());
    }
    canonical.operation = EditOperation::RemoveEmbeddedCircuit { id: source };
    Ok(())
}

/// How a linked mirror part relates to its source through its embedded circuit.
pub(super) enum MemberState {
    /// Linked: its source is the namespace's mirror image and a member of the source circuit.
    Linked,
    /// Its circuit is kept Local: the part is the mirror's own.
    Local,
    /// Its circuit is Omitted: the part has no geometry to edit.
    Omitted,
}

/// The state of `id`, a linked mirror part. A member of a linked embedded circuit must have the
/// namespace's mirror image as its source and that source must be a member of the source circuit.
pub(super) fn member_state(
    links: &Links<'_>,
    id: &str,
    link: Link<'_>,
) -> Result<MemberState, String> {
    let Some(circuit) = links
        .document
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.part_ids.iter().any(|member| member == id))
    else {
        return Ok(MemberState::Linked);
    };
    let Some(owner) = link
        .pair
        .halves
        .correspondences
        .iter()
        .find(|correspondence| {
            correspondence.kind == MirroredItemKind::EmbeddedCircuit
                && correspondence.mirror_id == circuit.id
        })
    else {
        return Ok(MemberState::Linked);
    };
    if let Some(overlay) = link.pair.halves.overlay_of(owner) {
        return Ok(match overlay.state {
            MirroredExceptionState::Local => MemberState::Local,
            MirroredExceptionState::Omitted => MemberState::Omitted,
        });
    }
    let source_circuit = links
        .document
        .embedded_circuits
        .iter()
        .find(|candidate| candidate.id == owner.source_id)
        .ok_or_else(|| format!("The source circuit {:?} is unavailable", owner.source_id))?;
    let source = link.correspondence.source_id.as_str();
    let (mirror_prefix, source_prefix) = (
        format!("embedded/{}/", circuit.id),
        format!("embedded/{}/", owner.source_id),
    );
    let expected = id
        .strip_prefix(&mirror_prefix)
        .map(|rest| format!("{source_prefix}{rest}"));
    if expected.as_deref() != Some(source)
        || !source_circuit
            .part_ids
            .iter()
            .any(|member| member == source)
    {
        return Err(format!(
            "{id:?} is a member of the linked embedded circuit {:?} but its source {source:?} is \
             not that circuit's member under {source_prefix:?}",
            circuit.id
        ));
    }
    Ok(MemberState::Linked)
}

/// Whether `id`, a linked mirror part, is translated as a linked part: a member of a Local
/// circuit executes as written; of an Omitted one, the request is refused.
pub(super) fn member_is_linked(
    links: &Links<'_>,
    id: &str,
    link: Link<'_>,
) -> Result<bool, String> {
    match member_state(links, id, link)? {
        MemberState::Linked => Ok(true),
        MemberState::Local => Ok(false),
        MemberState::Omitted => Err(Links::omitted(id)),
    }
}
