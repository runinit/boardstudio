//! Inverse-map translation of hosted, electrical and keycap requests (ticket 112).
//!
//! The matrix and part families of ticket 110 live in the parent module. This child module owns
//! the rest of the inventory 110 named: mounted modules, embedded circuits, nets, scan and wiring
//! modes and keycap settings, plus `MoveParts`/`RemoveParts` on circuit members (through
//! [`member_is_linked`]). It is private and inert: nothing calls it yet, 111 activates it and 115
//! composes it.
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
    EditOperation, MirroredExceptionState, MirroredItemKind, Net, PartDefinition, Pin, Pose2, Side,
    Vec2,
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
        Op::SetNet { net } => set_net(links, net, canonical)?,
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
        _ => unreachable!("only hosted, electrical and keycap families are routed here"),
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
/// `rotation`: the exact inverse of `derive::pose` (the heading rule is its own inverse).
fn source_pose(mapping: Pose2, at: Vec2, rotation: f64) -> Pose2 {
    Pose2 {
        at: pair::inverse(mapping, at),
        rotation: (pair::inverse_angle(mapping, rotation) + 180.0).rem_euclid(360.0),
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

    out.bus_id = if let Some(source) = unchanged(|a, b| a.bus_id == b.bus_id) {
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
    let pose = source_pose(pair.mapping, payload.at, payload.rotation);
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
    let mut source = source_module(links, pair, &source_id, instance, None, None)?;

    // The connector the operation creates takes `<module id>/vik-host-connector`: a mirror
    // module's connector is planned as the mirror of the source module's own.
    let creates_connector = connector.is_some()
        && instance.connection.as_ref().is_some_and(|connection| {
            connection.host_connector_part_id.is_empty()
                || connection.host_connector_part_id == generated(&instance.id)
        });
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
        if let Some(connection) = source.connection.as_mut()
            && !connection.host_connector_part_id.is_empty()
        {
            connection.host_connector_part_id = source_part.clone();
        }
        planned = Some(PartAddition {
            source_id: source_part,
            mirror_id: mirror_part,
            mirror_definition_id: connector.map(|item| item.id.clone()).unwrap_or_default(),
            mirror_reference: host_connector_reference(links, &pair.halves.mirror_board_id),
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

/// `modules::circuit::allocate_reference`, which is private to its module: the reference the
/// mirror board's circuit embed assigns a member.
fn allocate_reference(source: &str, used: &mut BTreeSet<String>) -> String {
    if !source.is_empty() && used.insert(source.to_owned()) {
        return source.to_owned();
    }
    let prefix = source.trim_end_matches(|c: char| c.is_ascii_digit());
    let prefix = if prefix.is_empty() { "U" } else { prefix };
    for index in 1.. {
        let candidate = format!("{prefix}{index}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("a finite circuit cannot exhaust component references")
}

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
    let Some(pair) = pair_for_mirror_board(links, host_board_id)? else {
        return Ok(());
    };
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

    // Joined host nets: the mirror host net a port joins is the source circuit's joined net's
    // mirror, so each join follows a Net correspondence.
    let mut source_joins = BTreeMap::new();
    let mut joined_pairs: BTreeMap<String, (String, String)> = BTreeMap::new();
    for (port, mirror_net) in joins {
        let source_net = followed(
            links,
            MirroredItemKind::Net,
            mirror_net,
            &format!("circuit {id}"),
            "joined host net",
        )?;
        if let Some(local) = circuit.ports.get(port) {
            joined_pairs.insert(local.clone(), (source_net.clone(), mirror_net.clone()));
        }
        source_joins.insert(port.clone(), source_net);
    }

    let (source_prefix, mirror_prefix) =
        (format!("embedded/{source_id}/"), format!("embedded/{id}/"));
    let mut member_ids = BTreeMap::new();
    let mut member_references = BTreeMap::new();
    let mut used_references: BTreeSet<String> = links
        .document
        .boards
        .iter()
        .find(|board| board.id == pair.halves.mirror_board_id)
        .into_iter()
        .flat_map(|board| board.part_ids.iter())
        .filter_map(|part_id| {
            links
                .document
                .parts
                .iter()
                .find(|part| &part.id == part_id)
                .map(|part| part.reference.clone())
        })
        .collect();
    for member in &circuit.parts {
        let source_member = format!("{source_prefix}{}", member.id);
        member_references.insert(
            source_member.clone(),
            allocate_reference(&member.reference, &mut used_references),
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

    let at = source_pose(pair.mapping, pose.at, pose.rotation);
    canonical.operation = EditOperation::EmbedModuleCircuit {
        id: source_id.clone(),
        definition: definition.clone(),
        host_board_id: pair.halves.source_board_id.clone(),
        pose: at,
        side: side.clone(),
        joins: source_joins,
    };
    canonical.hosted.circuits.push(CircuitAddition {
        source_id,
        mirror_id: id.to_owned(),
        member_ids,
        member_references,
        definition_ids,
        net_ids,
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

/// Whether `id`, a linked mirror part, is translated as a linked part. A member of a linked
/// embedded circuit is: its source member must be the namespace's mirror image and a member of
/// the source circuit. A member of a Local circuit executes as written; of an Omitted one, the
/// request is refused.
pub(super) fn member_is_linked(
    links: &Links<'_>,
    id: &str,
    link: Link<'_>,
) -> Result<bool, String> {
    let Some(circuit) = links
        .document
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.part_ids.iter().any(|member| member == id))
    else {
        return Ok(true);
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
        return Ok(true);
    };
    if let Some(overlay) = link.pair.halves.overlay_of(owner) {
        return match overlay.state {
            MirroredExceptionState::Local => Ok(false),
            MirroredExceptionState::Omitted => Err(Links::omitted(id)),
        };
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
    Ok(true)
}

// ---- nets -----------------------------------------------------------------------------------

/// A net's target state, following its owning circuit's exception for a private net.
fn net_target<'a>(links: &'a Links<'_>, id: &str) -> Result<Target<'a>, String> {
    let target = links.target(MirroredItemKind::Net, id)?;
    let Target::Linked(link) = target else {
        return Ok(target);
    };
    for owner in &link.pair.halves.correspondences {
        if owner.kind != MirroredItemKind::EmbeddedCircuit
            || !id.starts_with(&format!("embedded/{}/", owner.mirror_id))
        {
            continue;
        }
        if let Some(overlay) = link.pair.halves.overlay_of(owner) {
            return Ok(match overlay.state {
                MirroredExceptionState::Local => Target::Local,
                MirroredExceptionState::Omitted => Target::Omitted,
            });
        }
    }
    Ok(Target::Linked(link))
}

/// A mirror net edit is a source net edit of the pins that moved. Pins on the mirror's own parts
/// (no source counterpart) are not reflected, so the request must leave them as they are.
fn set_net(links: &Links<'_>, net: &Net, canonical: &mut Canonical) -> Result<(), String> {
    let link = match net_target(links, &net.id)? {
        Target::NotMirror | Target::Local => return Ok(()),
        Target::Omitted => return Err(Links::omitted(&net.id)),
        Target::Linked(link) => link,
    };
    let find = |id: &str| {
        links
            .document
            .nets
            .iter()
            .find(|candidate| candidate.id == id)
            .ok_or_else(|| format!("The net {id:?} is unavailable"))
    };
    let current_mirror = find(&net.id)?;
    let current_source = find(&link.correspondence.source_id)?;

    // A pin splits into the pin it is on the source half, or the mirror's own pin.
    let split = |pins: &[Pin]| -> Result<(Vec<Pin>, Vec<Pin>), String> {
        let (mut mapped, mut own) = (Vec::new(), Vec::new());
        for pin in pins {
            match links.target(MirroredItemKind::Part, &pin.part_id)? {
                Target::Linked(owner) => mapped.push(Pin {
                    part_id: owner.correspondence.source_id.clone(),
                    pad_id: pin.pad_id.clone(),
                }),
                Target::NotMirror | Target::Local | Target::Omitted => own.push(pin.clone()),
            }
        }
        Ok((mapped, own))
    };
    let (payload_mapped, payload_own) = split(&net.pins)?;
    let (current_mapped, current_own) = split(&current_mirror.pins)?;
    let same_own = payload_own.len() == current_own.len()
        && payload_own.iter().all(|pin| current_own.contains(pin));
    if !same_own {
        return Err(format!(
            "The net {:?} changes pins on parts that have no source on the other half; those \
             pins are not reflected, so the edit cannot travel through the source net",
            net.id
        ));
    }

    let mut pins: Vec<Pin> = current_source
        .pins
        .iter()
        .filter(|pin| !current_mapped.contains(pin) || payload_mapped.contains(pin))
        .cloned()
        .collect();
    for pin in payload_mapped {
        if !pins.contains(&pin) {
            pins.push(pin);
        }
    }
    let name = if net.name == current_mirror.name {
        current_source.name.clone()
    } else {
        net.name.clone()
    };
    canonical.operation = EditOperation::SetNet {
        net: Net {
            id: link.correspondence.source_id.clone(),
            name,
            pins,
        },
    };
    Ok(())
}
