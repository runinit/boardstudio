//! Durable identity floors, counters and the mechanical identity ledger.

use super::super::change::MechanicalTarget;
use super::feature_lifecycle::feature_ids;
use crate::model::*;
use std::collections::BTreeMap;

/// The reserved high-water marks an edit's allocation must not hand out again.
#[derive(Clone, Copy, Default)]
pub struct IdentityFloor {
    pub(super) opening: u64,
    pub(super) vertex: u64,
    pub(super) critical_fit: u64,
    pub(super) hardware: u64,
    pub(super) mount: u64,
}

impl IdentityFloor {
    pub(super) fn merged(self, other: Self) -> Self {
        Self {
            opening: self.opening.max(other.opening),
            vertex: self.vertex.max(other.vertex),
            critical_fit: self.critical_fit.max(other.critical_fit),
            hardware: self.hardware.max(other.hardware),
            mount: self.mount.max(other.mount),
        }
    }
}

pub(super) fn exhausted_identity(kind: &str) -> String {
    format!("{kind} identity is exhausted.")
}

/// Allocates the next monotonic identity from `counter`.
///
/// `counter` is the next free number; `u64::MAX` means the namespace can never
/// advance again. A live or reserved `MAX` is refused rather than saturated into a
/// reusable identity.
pub(super) fn reserve_identity(
    counter: &mut u64,
    highest: u64,
    floor: u64,
    prefix: &str,
    kind: &str,
) -> Result<String, String> {
    if *counter == u64::MAX || highest == u64::MAX || floor == u64::MAX {
        return Err(exhausted_identity(kind));
    }
    let next = (*counter).max(1).max(highest + 1).max(floor + 1);
    *counter = next.saturating_add(1);
    Ok(format!("{prefix}-{next}"))
}

/// Reserves the next authored opening or vertex identity for a configuration.
///
/// The stored reservation is monotonic, so a removed opening or vertex identity is
/// never handed out again and an intent queued against it can never reach a later
/// addition. The first reservation for a configuration that predates the field
/// normalizes to the highest identity it already carries, which preserves existing
/// geometry and ids.
pub(super) fn reserve_opening_identity(
    configuration: &mut MechanicalConfiguration,
    floor: u64,
) -> Result<String, String> {
    let highest = configuration
        .openings
        .as_ref()
        .into_iter()
        .flatten()
        .map(|opening| identity_index(&opening.id))
        .max()
        .unwrap_or(0);
    reserve_identity(
        &mut configuration.next_opening_identity,
        highest,
        floor,
        "opening",
        "Opening",
    )
}

pub(super) fn reserve_vertex_identity(
    opening: &mut MechanicalOpening,
    floor: u64,
) -> Result<String, String> {
    let highest = opening
        .vertices
        .iter()
        .map(|vertex| identity_index(&vertex.id))
        .max()
        .unwrap_or(0);
    reserve_identity(
        &mut opening.next_vertex_identity,
        highest,
        floor,
        "vertex",
        "Vertex",
    )
}

/// The identities a target's member reserves for its pair-owned features.
///
/// A linked feature is owned by the pair and is invisible to the member's raw
/// configuration, and stays reserved while the member omits it. Reserving from those
/// identities keeps a new local addition from taking one the pair owns for this member,
/// which would otherwise let a rejoin produce two openings under one identity or let a
/// queued intent reach the wrong opening.
pub(super) fn reserved_identity_floor(
    document: &ProjectDoc,
    target: &MechanicalTarget,
) -> IdentityFloor {
    let MechanicalTarget::Instance { instance_id } = target else {
        return IdentityFloor::default();
    };
    let Some(hardware) = document.hardware.as_ref() else {
        return IdentityFloor::default();
    };
    let member_features = || {
        hardware
            .pairs
            .iter()
            .flat_map(|pair| pair.features.iter())
            .filter_map(move |feature| {
                feature
                    .members
                    .iter()
                    .find(|member| member.instance_id == *instance_id)
                    .map(|member| (feature, member))
            })
    };
    let floor = |prefix: &str| {
        member_features()
            .filter_map(|(_, member)| identity_index_for(&member.feature_id, prefix))
            .max()
            .unwrap_or(0)
    };
    let mount = member_features()
        .filter(|(feature, _)| {
            matches!(
                feature.kind,
                MechanicalFeatureKind::Mount | MechanicalFeatureKind::ClosureMount
            )
        })
        .filter_map(|(feature, member)| {
            let mount_id = match &feature.value {
                MechanicalFeatureValue::Mount { mount } => Some(mount.id.as_str()),
                _ => None,
            };
            identity_index_for(&member.feature_id, "case-mechanical-mount")
                .into_iter()
                .chain(
                    mount_id
                        .into_iter()
                        .filter_map(|id| identity_index_for(id, "case-mechanical-mount")),
                )
                .max()
        })
        .max()
        .unwrap_or(0);
    // A half that outlived its own configuration (a disabled half) keeps the highest
    // identity it ever reserved, so re-enabling cannot reuse a removed one.
    let stored = hardware
        .instances
        .iter()
        .find(|instance| instance.id == *instance_id)
        .and_then(|instance| instance.mechanical_identity_reservation)
        .unwrap_or_default();
    IdentityFloor {
        opening: floor("opening").max(reservation_floor(stored.next_opening_identity)),
        vertex: floor("vertex").max(reservation_floor(stored.next_vertex_identity)),
        critical_fit: reservation_floor(stored.next_critical_fit_identity),
        hardware: reservation_floor(stored.next_hardware_identity),
        mount: mount.max(reservation_floor(stored.next_mount_identity)),
    }
}

/// A stored `next == MAX` is the exhausted sentinel: no further identity is handed out.
pub(super) fn reservation_floor(next: u64) -> u64 {
    if next == u64::MAX {
        u64::MAX
    } else {
        next.saturating_sub(1)
    }
}

/// Who owns a mechanical configuration's identity namespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum IdentityConfigOwner {
    Board(String),
    Instance(String),
}

/// The per-opening vertex namespace. Vertex identities only need to stay unique
/// within their owning opening, so two openings may both use `vertex-1`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct IdentityVertexOwner {
    pub(super) config: IdentityConfigOwner,
    pub(super) opening_id: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct IdentityCounter {
    pub(super) next: u64,
    pub(super) exhausted: bool,
}

impl IdentityCounter {
    pub(super) fn merged(self, other: Self) -> Self {
        Self {
            next: self.next.max(other.next),
            exhausted: self.exhausted || other.exhausted,
        }
    }

    pub(super) fn floor(self) -> u64 {
        if self.exhausted {
            u64::MAX
        } else {
            self.next.saturating_sub(1)
        }
    }

    /// A live `MAX` identity leaves no further number; a stored `MAX` counter
    /// without a live `MAX` still has `MAX` available.
    pub(super) fn captured(stored: u64, highest: u64) -> Self {
        Self {
            next: stored.max(highest.saturating_add(1)),
            exhausted: stored == u64::MAX || highest == u64::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct IdentityCounters {
    pub(super) opening: IdentityCounter,
    pub(super) vertex: IdentityCounter,
    pub(super) critical_fit: IdentityCounter,
    pub(super) hardware: IdentityCounter,
    pub(super) mount: IdentityCounter,
}

impl IdentityCounters {
    pub(super) fn merged(self, other: Self) -> Self {
        Self {
            opening: self.opening.merged(other.opening),
            vertex: self.vertex.merged(other.vertex),
            critical_fit: self.critical_fit.merged(other.critical_fit),
            hardware: self.hardware.merged(other.hardware),
            mount: self.mount.merged(other.mount),
        }
    }

    pub(super) fn from_reservation(reservation: MechanicalIdentityReservation) -> Self {
        Self {
            opening: IdentityCounter {
                next: reservation.next_opening_identity,
                exhausted: reservation.next_opening_identity == u64::MAX,
            },
            vertex: IdentityCounter {
                next: reservation.next_vertex_identity,
                exhausted: reservation.next_vertex_identity == u64::MAX,
            },
            critical_fit: IdentityCounter {
                next: reservation.next_critical_fit_identity,
                exhausted: reservation.next_critical_fit_identity == u64::MAX,
            },
            hardware: IdentityCounter {
                next: reservation.next_hardware_identity,
                exhausted: reservation.next_hardware_identity == u64::MAX,
            },
            mount: IdentityCounter {
                next: reservation.next_mount_identity,
                exhausted: reservation.next_mount_identity == u64::MAX,
            },
        }
    }
}

/// Session high-water marks for mechanical identities that outlive Undo snapshots,
/// disabled configurations and temporarily absent owners.
///
/// Core owns allocation; this ledger only raises reservations and never mints an
/// identity. A successful explicit Open starts a new ledger (a new document session);
/// failed opens, previews, failed edits, undo/redo, panel remounts and disable/re-enable
/// keep it.
#[derive(Clone, Debug, Default)]
pub struct MechanicalIdentityLedger {
    pub(super) configs: BTreeMap<IdentityConfigOwner, IdentityCounters>,
    pub(super) vertices: BTreeMap<IdentityVertexOwner, IdentityCounter>,
}

impl MechanicalIdentityLedger {
    /// A ledger seeded from every live identity and stored counter in `document`.
    pub fn seeded(document: &ProjectDoc) -> Self {
        let mut ledger = Self::default();
        ledger.capture(document);
        ledger
    }

    /// Raises the ledger to cover every identity and counter currently in `document`.
    pub fn capture(&mut self, document: &ProjectDoc) {
        if let Some(configuration) = &document.mechanical {
            self.capture_config(
                IdentityConfigOwner::Board(configuration.board_id.clone()),
                configuration,
            );
        }
        let Some(hardware) = &document.hardware else {
            return;
        };
        for instance in &hardware.instances {
            let owner = IdentityConfigOwner::Instance(instance.id.clone());
            if let Some(configuration) = &instance.mechanical {
                self.capture_config(owner.clone(), configuration);
            }
            if let Some(reservation) = instance.mechanical_identity_reservation {
                let counters = IdentityCounters::from_reservation(reservation);
                self.configs
                    .entry(owner)
                    .and_modify(|entry| *entry = entry.merged(counters))
                    .or_insert(counters);
            }
        }
        // A linked opening is stored only in the pair's authored value; its vertex
        // counter still belongs to each member's vertex namespace.
        for pair in &hardware.pairs {
            for feature in &pair.features {
                let MechanicalFeatureValue::Opening { opening } = &feature.value else {
                    continue;
                };
                let counter = IdentityCounter::captured(
                    opening.next_vertex_identity,
                    opening
                        .vertices
                        .iter()
                        .map(|vertex| identity_index(&vertex.id))
                        .max()
                        .unwrap_or(0),
                );
                for member in &feature.members {
                    let key = IdentityVertexOwner {
                        config: IdentityConfigOwner::Instance(member.instance_id.clone()),
                        opening_id: member.feature_id.clone(),
                    };
                    self.vertices
                        .entry(key)
                        .and_modify(|entry| *entry = entry.merged(counter))
                        .or_insert(counter);
                }
            }
        }
    }

    pub(super) fn capture_config(
        &mut self,
        owner: IdentityConfigOwner,
        configuration: &MechanicalConfiguration,
    ) {
        let highest_opening = feature_ids(configuration, MechanicalFeatureKind::Opening)
            .into_iter()
            .map(|id| identity_index(&id))
            .max()
            .unwrap_or(0);
        let highest_vertex = configuration
            .openings
            .iter()
            .flatten()
            .flat_map(|opening| opening.vertices.iter())
            .map(|vertex| identity_index(&vertex.id))
            .max()
            .unwrap_or(0);
        let highest_critical_fit = configuration
            .critical_fits
            .iter()
            .flatten()
            .filter_map(|fit| identity_index_for(&fit.id, "case-critical-fit"))
            .max()
            .unwrap_or(0);
        let highest_hardware = configuration
            .hardware
            .iter()
            .flatten()
            .filter_map(|entry| identity_index_for(&entry.id, "case-mechanical-hardware"))
            .max()
            .unwrap_or(0);
        let highest_mount = feature_ids(configuration, MechanicalFeatureKind::Mount)
            .into_iter()
            .chain(feature_ids(
                configuration,
                MechanicalFeatureKind::ClosureMount,
            ))
            .filter_map(|id| identity_index_for(&id, "case-mechanical-mount"))
            .max()
            .unwrap_or(0);
        let counters = IdentityCounters {
            opening: IdentityCounter::captured(
                configuration.next_opening_identity,
                highest_opening,
            ),
            vertex: IdentityCounter::captured(
                configuration
                    .openings
                    .iter()
                    .flatten()
                    .map(|opening| opening.next_vertex_identity)
                    .max()
                    .unwrap_or(0),
                highest_vertex,
            ),
            critical_fit: IdentityCounter::captured(
                configuration.next_critical_fit_identity,
                highest_critical_fit,
            ),
            hardware: IdentityCounter::captured(
                configuration.next_hardware_identity,
                highest_hardware,
            ),
            mount: IdentityCounter::captured(configuration.next_mount_identity, highest_mount),
        };
        self.configs
            .entry(owner.clone())
            .and_modify(|entry| *entry = entry.merged(counters))
            .or_insert(counters);
        for opening in configuration.openings.iter().flatten() {
            let key = IdentityVertexOwner {
                config: owner.clone(),
                opening_id: opening.id.clone(),
            };
            let counter = IdentityCounter::captured(
                opening.next_vertex_identity,
                opening
                    .vertices
                    .iter()
                    .map(|vertex| identity_index(&vertex.id))
                    .max()
                    .unwrap_or(0),
            );
            self.vertices
                .entry(key)
                .and_modify(|entry| *entry = entry.merged(counter))
                .or_insert(counter);
        }
    }

    /// The session high-water floors for one target's allocation.
    ///
    /// Allocation merges these with the document's live identities and persisted
    /// counters, so Undo, a disabled configuration or a temporarily absent owner
    /// cannot make a removed identity available again.
    pub fn floors(&self, target: &MechanicalTarget) -> IdentityFloor {
        let owner = match target {
            MechanicalTarget::Board { board_id } => IdentityConfigOwner::Board(board_id.clone()),
            MechanicalTarget::Instance { instance_id } => {
                IdentityConfigOwner::Instance(instance_id.clone())
            }
        };
        let counters = self.configs.get(&owner).copied().unwrap_or_default();
        let vertex = self
            .vertices
            .iter()
            .filter(|(key, _)| key.config == owner)
            .map(|(_, counter)| counter.floor())
            .max()
            .unwrap_or(0)
            .max(counters.vertex.floor());
        IdentityFloor {
            opening: counters.opening.floor(),
            vertex,
            critical_fit: counters.critical_fit.floor(),
            hardware: counters.hardware.floor(),
            mount: counters.mount.floor(),
        }
    }
}

/// The highest opening and vertex identity a configuration can already hand out.
pub(super) fn identity_reservation_of(
    configuration: &MechanicalConfiguration,
) -> MechanicalIdentityReservation {
    let openings = configuration.openings.as_ref();
    let highest_opening = feature_ids(configuration, MechanicalFeatureKind::Opening)
        .into_iter()
        .map(|id| identity_index(&id))
        .max()
        .unwrap_or(0);
    let highest_vertex = openings
        .into_iter()
        .flatten()
        .flat_map(|opening| opening.vertices.iter())
        .map(|vertex| identity_index(&vertex.id))
        .max()
        .unwrap_or(0);
    MechanicalIdentityReservation {
        next_opening_identity: configuration
            .next_opening_identity
            .max(highest_opening.saturating_add(1)),
        next_vertex_identity: openings
            .into_iter()
            .flatten()
            .map(|opening| opening.next_vertex_identity)
            .max()
            .unwrap_or(0)
            .max(highest_vertex.saturating_add(1)),
        next_critical_fit_identity: configuration.next_critical_fit_identity.max(
            configuration
                .critical_fits
                .as_ref()
                .into_iter()
                .flatten()
                .map(|fit| identity_index_for(&fit.id, "case-critical-fit").unwrap_or(0))
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        ),
        next_hardware_identity: configuration.next_hardware_identity.max(
            configuration
                .hardware
                .as_ref()
                .into_iter()
                .flatten()
                .map(|entry| identity_index_for(&entry.id, "case-mechanical-hardware").unwrap_or(0))
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        ),
        next_mount_identity: configuration.next_mount_identity.max(
            feature_ids(configuration, MechanicalFeatureKind::Mount)
                .into_iter()
                .chain(feature_ids(
                    configuration,
                    MechanicalFeatureKind::ClosureMount,
                ))
                .filter_map(|id| identity_index_for(&id, "case-mechanical-mount"))
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        ),
    }
}

/// Raises a materialized configuration's reservations to cover pair-owned identities.
///
/// Used when ownership ends and a half keeps the model it showed: the persisted counters
/// then cover every identity that was visible through the pair, so a later local addition
/// cannot take one of them.
pub(super) fn merge_identity_floor(
    configuration: &mut MechanicalConfiguration,
    floor: IdentityFloor,
) -> Result<(), String> {
    let raised = |current: u64, floor: u64, kind: &str| -> Result<u64, String> {
        if floor == u64::MAX {
            if current == u64::MAX {
                return Ok(current);
            }
            return Err(exhausted_identity(kind));
        }
        Ok(current.max(floor + 1))
    };
    configuration.next_opening_identity = raised(
        configuration.next_opening_identity,
        floor.opening,
        "Opening",
    )?;
    if floor.critical_fit > 0 {
        configuration.next_critical_fit_identity = raised(
            configuration.next_critical_fit_identity,
            floor.critical_fit,
            "Critical fit",
        )?;
    }
    if floor.hardware > 0 {
        configuration.next_hardware_identity = raised(
            configuration.next_hardware_identity,
            floor.hardware,
            "Hardware",
        )?;
    }
    // Live mounts already seed allocation from their highest ordinal; only a
    // reserved mount that is not visible here needs a persisted counter.
    let highest_live_mount = configuration
        .mounts
        .iter()
        .chain(configuration.closure_mounts.iter().flatten())
        .filter_map(|mount| identity_index_for(&mount.id, "case-mechanical-mount"))
        .max()
        .unwrap_or(0);
    if floor.mount > 0 && floor.mount > highest_live_mount {
        configuration.next_mount_identity =
            raised(configuration.next_mount_identity, floor.mount, "Mount")?;
    }
    if let Some(openings) = configuration.openings.as_mut() {
        for opening in openings {
            opening.next_vertex_identity =
                raised(opening.next_vertex_identity, floor.vertex, "Vertex")?;
        }
    }
    Ok(())
}

/// The numeric suffix of a `prefix-N` identity, or `None` for another prefix.
pub(super) fn identity_index_for(id: &str, prefix: &str) -> Option<u64> {
    id.strip_prefix(prefix)?.strip_prefix('-')?.parse().ok()
}

/// The numeric suffix of an `opening-N`/`vertex-N` identity, or zero.
pub(super) fn identity_index(id: &str) -> u64 {
    id.rsplit('-')
        .next()
        .and_then(|index| index.parse().ok())
        .unwrap_or(0)
}
