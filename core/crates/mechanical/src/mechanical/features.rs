//! Mechanical mirror pairs and linked features.
//!
//! Owns the persisted pair/feature rules: pair membership and mapping validation,
//! the shared-construction subset, per-feature linked/local/omitted states, and
//! counterpart geometry derived from the single authored value. Generated
//! clearance parts remain projections, not a second relationship store.
//!
//! # Rules enforced here
//!
//! * A pair names exactly two distinct physical instances that exist in
//!   [`ProjectDoc::hardware`], pair ids are unique in the document, and an
//!   instance belongs to at most one pair.
//! * A pair mapping is finite. A feature's kind must match its value variant
//!   (mounts and closure mounts share [`MechanicalFeatureValue::Mount`]), its
//!   relationship id is unique within the pair, and its `author` is one of the
//!   two members.
//! * Every feature carries exactly one member record per pair member, with a
//!   non-empty member feature id that is unique for that instance within the
//!   pair. The authoring member's record is [`MechanicalFeatureState::Linked`]:
//!   the stored value is the linked value, while `Local` and `Omitted` records
//!   are per-half exceptions.
//! * Construction consistency: a member that owns its construction locally
//!   ([`MechanicalConstructionOwnership::Local`]) must already store the pair's
//!   shared subset, because that member's own configuration stays authoritative.
//!   A [`MechanicalConstructionOwnership::Pair`] member takes the pair
//!   construction as authority, so its stored shared subset is only an overlay
//!   base and may differ. [`MechanicalConstructionOwnership::Legacy`] members
//!   resolve their shared subset through the legacy overlay in [`super::inputs`],
//!   which this module does not re-derive, so they are not compared here.
//!
//! # Stored frames
//!
//! A feature's authored value is stored in its authoring member's Case frame. The
//! other half's geometry is derived on demand by [`counterpart`] and never
//! persisted as a second writable value.

use crate::model::*;

mod validation;
pub use validation::validate;

/// The other half's derived geometry for a linked feature.
#[derive(Clone, Debug, PartialEq)]
pub struct Counterpart {
    /// The member that receives the derived geometry.
    pub instance_id: String,
    /// That member's own feature id for this relationship.
    pub feature_id: String,
    /// The derived geometry in that member's Case frame.
    pub value: MechanicalFeatureValue,
}

/// The direction a counterpart is derived in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    /// From the first pair member to the second, using the stored mapping.
    Forward,
    /// From the second pair member back to the first, using the exact inverse.
    Inverse,
}

impl Direction {
    fn point(self, mapping: Pose2, point: Vec2) -> Vec2 {
        match self {
            Direction::Forward => super::pair::forward(mapping, point),
            Direction::Inverse => super::pair::inverse(mapping, point),
        }
    }

    fn pose(self, mapping: Pose2) -> Pose2 {
        match self {
            Direction::Forward => mapping,
            Direction::Inverse => inverse_pose(mapping),
        }
    }
}

pub use boardstudio_model::mechanical::features::inverse_pose;

/// Derives the other half's geometry for a linked feature.
///
/// The authored value lives in its authoring member's Case frame. When
/// `from_instance_id` is the author, the counterpart is the other member and its
/// geometry is the authored value mapped through the pair map: mounts and closure
/// mounts map their `at` point, authored openings go through
/// [`super::pair::reflect_opening`] (vertex ids preserved, winding reversed,
/// `z`/`height` preserved), and gasket anchors are carried over unchanged because
/// their region association is re-derived against the counterpart's own contours.
/// When `from_instance_id` is the non-authoring linked half, the counterpart is
/// the author and its geometry is the stored authored value itself.
///
/// Returns `Ok(None)` when either half is `Local` or `Omitted`: a local half owns
/// its own geometry and an omitted half has none, so neither has a derived
/// counterpart. Errors when the instance is not a pair member or a member record
/// is missing.
pub fn counterpart(
    pair: &MechanicalMirrorPair,
    feature: &MechanicalFeature,
    from_instance_id: &str,
) -> Result<Option<Counterpart>, String> {
    let from_index = pair
        .members
        .iter()
        .position(|member| member == from_instance_id)
        .ok_or_else(|| {
            format!(
                "physical instance {from_instance_id:?} is not a member of mechanical pair {:?}",
                pair.id
            )
        })?;
    let author_index = pair
        .members
        .iter()
        .position(|member| member == &feature.author)
        .ok_or_else(|| {
            format!(
                "mechanical feature {:?} authors from {:?}, which is not a member of pair {:?}",
                feature.id, feature.author, pair.id
            )
        })?;
    let other_id = &pair.members[1 - from_index];
    let from_member = feature_member(feature, from_instance_id).ok_or_else(|| {
        format!(
            "mechanical feature {:?} has no member record for {from_instance_id:?}",
            feature.id
        )
    })?;
    let other_member = feature_member(feature, other_id).ok_or_else(|| {
        format!(
            "mechanical feature {:?} has no member record for {other_id:?}",
            feature.id
        )
    })?;
    if from_member.state != MechanicalFeatureState::Linked
        || other_member.state != MechanicalFeatureState::Linked
    {
        return Ok(None);
    }
    // The authored value lives in the author's frame, so mapping it out uses the
    // direction of the authoring member, not of the caller.
    let value = if from_index == author_index {
        let direction = if from_index == 0 {
            Direction::Forward
        } else {
            Direction::Inverse
        };
        map_value(pair.mapping, &feature.value, direction)
    } else {
        feature.value.clone()
    };
    Ok(Some(Counterpart {
        instance_id: other_id.clone(),
        feature_id: other_member.feature_id.clone(),
        value,
    }))
}

/// The member record a linked feature carries for one half, by durable id.
pub fn feature_member<'a>(
    feature: &'a MechanicalFeature,
    instance_id: &str,
) -> Option<&'a MechanicalFeatureMember> {
    feature
        .members
        .iter()
        .find(|member| member.instance_id == instance_id)
}

fn map_value(
    mapping: Pose2,
    value: &MechanicalFeatureValue,
    direction: Direction,
) -> MechanicalFeatureValue {
    match value {
        MechanicalFeatureValue::Mount { mount } => MechanicalFeatureValue::Mount {
            mount: Mount {
                at: direction.point(mapping, mount.at),
                ..mount.clone()
            },
        },
        MechanicalFeatureValue::Opening { opening } => MechanicalFeatureValue::Opening {
            opening: super::pair::reflect_opening(direction.pose(mapping), opening),
        },
        // The anchor keeps its identity, region association and fraction. Its
        // resolved placement is mirrored by `mirror_support` once the counterpart
        // has been resolved against that member's own contours.
        MechanicalFeatureValue::GasketSupport { anchor } => MechanicalFeatureValue::GasketSupport {
            anchor: anchor.clone(),
        },
    }
}

/// Mirrors a resolved gasket support through the pair map.
///
/// The support's `at` point maps through the pair map and its `tangent`/`normal`
/// through the linear part, so the reflected support keeps a consistent frame and
/// the contour's winding; `length`, `width`, `z`, `thickness`, identity, region
/// association and placement ownership are preserved. Pass
/// [`inverse_pose`]`(pair.mapping)` to mirror from the second member back to the
/// first.
pub fn mirror_support(
    mapping: Pose2,
    support: &MechanicalGasketSupport,
) -> MechanicalGasketSupport {
    MechanicalGasketSupport {
        at: super::pair::forward(mapping, support.at),
        tangent: super::pair::forward_vector(mapping, support.tangent),
        normal: super::pair::forward_vector(mapping, support.normal),
        ..support.clone()
    }
}

/// The shared construction subset of a mechanical configuration.
///
/// Shared by a pair: plate/case `method` and `mount`, `integrated_plate_frame`,
/// `bottom_style`, `middle_frame`, the dimensional and foam settings
/// (`plate_thickness`, `plate_foam_thickness`, `bottom_foam_thickness`,
/// `bottom_thickness`, `plate_to_pcb`, `wall_thickness`, `clearance`), the common
/// gasket dimensions/material (`gasket`, `gasket_travel`, `opening_allowance`,
/// `internal_gasket`), manufacturing processes (`part_processes`) and the gasket
/// layout's common settings.
///
/// Local to each member: `board_id`/`pcb_thickness`, `battery`/`battery_height`,
/// `mounts`, `closure_mounts`, `openings`, `stabilizers`, `profiles`,
/// `hardware` and `critical_fits`. The gasket layout's `supports` are local too,
/// because each half resolves its own anchors against its own contours; they are
/// not captured here.
pub fn shared_construction(configuration: &MechanicalConfiguration) -> MechanicalConstruction {
    MechanicalConstruction {
        method: configuration.method.clone(),
        mount: configuration.mount.clone(),
        integrated_plate_frame: configuration.integrated_plate_frame,
        bottom_style: configuration.bottom_style.clone(),
        middle_frame: configuration.middle_frame,
        plate_thickness: configuration.plate_thickness,
        plate_foam_thickness: configuration.plate_foam_thickness,
        bottom_foam_thickness: configuration.bottom_foam_thickness,
        bottom_thickness: configuration.bottom_thickness,
        plate_to_pcb: configuration.plate_to_pcb,
        wall_thickness: configuration.wall_thickness,
        clearance: configuration.clearance,
        gasket: configuration.gasket.clone(),
        gasket_travel: configuration.gasket_travel,
        opening_allowance: configuration.opening_allowance,
        part_processes: configuration.part_processes.clone(),
        internal_gasket: configuration.internal_gasket.clone(),
        gasket_layout: shared_gasket_layout(configuration.gasket_layout.as_ref()),
    }
}

fn shared_gasket_layout(layout: Option<&MechanicalGasketLayout>) -> Option<MechanicalGasketLayout> {
    layout.map(|layout| {
        let mut shared = layout.clone();
        shared.supports.clear();
        shared
    })
}

/// Writes a pair's shared construction subset onto a member configuration.
///
/// Only the fields captured by [`shared_construction`] are written. Board
/// identity and thickness, batteries, mounts, closure mounts, openings,
/// stabilizers, profiles, hardware specifications and critical fits are left
/// untouched, and the member's own gasket supports are preserved. An absent
/// optional field in `construction` leaves the member's stored value in place,
/// matching the pair read path in [`super::inputs`].
pub fn apply_construction(
    configuration: &mut MechanicalConfiguration,
    construction: &MechanicalConstruction,
) {
    configuration.method = construction.method.clone();
    configuration.mount = construction.mount.clone();
    configuration.integrated_plate_frame = construction.integrated_plate_frame;
    configuration.plate_thickness = construction.plate_thickness;
    configuration.plate_foam_thickness = construction.plate_foam_thickness;
    configuration.bottom_foam_thickness = construction.bottom_foam_thickness;
    configuration.bottom_thickness = construction.bottom_thickness;
    configuration.plate_to_pcb = construction.plate_to_pcb;
    configuration.wall_thickness = construction.wall_thickness;
    configuration.clearance = construction.clearance;
    if construction.bottom_style.is_some() {
        configuration.bottom_style = construction.bottom_style.clone();
    }
    if construction.middle_frame.is_some() {
        configuration.middle_frame = construction.middle_frame;
    }
    if construction.gasket.is_some() {
        configuration.gasket = construction.gasket.clone();
    }
    if construction.gasket_travel.is_some() {
        configuration.gasket_travel = construction.gasket_travel;
    }
    if construction.opening_allowance.is_some() {
        configuration.opening_allowance = construction.opening_allowance;
    }
    if construction.part_processes.is_some() {
        configuration.part_processes = construction.part_processes.clone();
    }
    if construction.internal_gasket.is_some() {
        configuration.internal_gasket = construction.internal_gasket.clone();
    }
    if let Some(layout) = construction.gasket_layout.as_ref() {
        let mut applied = layout.clone();
        applied.supports = configuration
            .gasket_layout
            .as_ref()
            .map(|local| local.supports.clone())
            .unwrap_or_default();
        configuration.gasket_layout = Some(applied);
    }
}

/// The state a member holds for one of its features, addressed by the member's
/// own feature id. `None` when the pair has no such member feature.
pub fn member_state(
    pair: &MechanicalMirrorPair,
    instance_id: &str,
    feature_id: &str,
) -> Option<MechanicalFeatureState> {
    pair.features
        .iter()
        .flat_map(|feature| feature.members.iter())
        .find(|member| member.instance_id == instance_id && member.feature_id == feature_id)
        .map(|member| member.state)
}

/// Sets a member feature's state, the data-level Link/Rejoin, MakeLocal and Omit.
pub fn set_member_state(
    pair: &mut MechanicalMirrorPair,
    instance_id: &str,
    feature_id: &str,
    state: MechanicalFeatureState,
) -> Result<(), String> {
    for feature in &mut pair.features {
        for member in &mut feature.members {
            if member.instance_id == instance_id && member.feature_id == feature_id {
                member.state = state;
                return Ok(());
            }
        }
    }
    Err(format!(
        "mechanical pair {:?} has no member feature {feature_id:?} for physical instance {instance_id:?}",
        pair.id
    ))
}

/// Adds a linked feature with a `Linked` record for both pair members.
///
/// The member feature id is the authored value's own id (the mount, opening or
/// anchor id), which both halves keep when geometry is reflected. Adding a
/// feature to only one half is a `Local` exception on an existing relationship
/// ([`make_local`]) or an ordinary local feature in that instance's own
/// configuration; it is not a pair record.
pub fn add_linked_feature(
    pair: &mut MechanicalMirrorPair,
    mut feature: MechanicalFeature,
) -> Result<(), String> {
    if !pair.members.contains(&feature.author) {
        return Err(format!(
            "mechanical feature {:?} authors from {:?}, which is not a member of pair {:?}",
            feature.id, feature.author, pair.id
        ));
    }
    if pair
        .features
        .iter()
        .any(|existing| existing.id == feature.id)
    {
        return Err(format!(
            "mechanical pair {:?} already has a feature {:?}",
            pair.id, feature.id
        ));
    }
    let member_feature_id = value_id(&feature.value);
    if member_feature_id.trim().is_empty() {
        return Err(format!(
            "mechanical feature {:?} of pair {:?} has an authored value without an id",
            feature.id, pair.id
        ));
    }
    feature.members = pair
        .members
        .iter()
        .map(|instance_id| MechanicalFeatureMember {
            instance_id: instance_id.clone(),
            feature_id: member_feature_id.into(),
            state: MechanicalFeatureState::Linked,
        })
        .collect();
    pair.features.push(feature);
    Ok(())
}

fn value_id(value: &MechanicalFeatureValue) -> &str {
    match value {
        MechanicalFeatureValue::Mount { mount } => &mount.id,
        MechanicalFeatureValue::Opening { opening } => &opening.id,
        MechanicalFeatureValue::GasketSupport { anchor } => &anchor.id,
    }
}

/// Materializes a member's own geometry for a linked feature (Make local).
///
/// The caller copies the currently derived geometry into the instance's ordinary
/// configuration; this records the exception so the other half keeps deriving
/// from the stored linked value.
pub fn make_local(
    pair: &mut MechanicalMirrorPair,
    instance_id: &str,
    feature_id: &str,
) -> Result<(), String> {
    set_member_state(pair, instance_id, feature_id, MechanicalFeatureState::Local)
}

/// Records an omission on one half (Remove on this half).
pub fn omit_member(
    pair: &mut MechanicalMirrorPair,
    instance_id: &str,
    feature_id: &str,
) -> Result<(), String> {
    set_member_state(
        pair,
        instance_id,
        feature_id,
        MechanicalFeatureState::Omitted,
    )
}

/// Clears one half's local/omitted exception, re-deriving from the stored value.
pub fn rejoin_member(
    pair: &mut MechanicalMirrorPair,
    instance_id: &str,
    feature_id: &str,
) -> Result<(), String> {
    set_member_state(
        pair,
        instance_id,
        feature_id,
        MechanicalFeatureState::Linked,
    )
}

/// Removes a linked feature with both member records (linked Delete).
pub fn remove_linked_feature(
    pair: &mut MechanicalMirrorPair,
    feature_id: &str,
) -> Result<(), String> {
    let index = pair
        .features
        .iter()
        .position(|feature| feature.id == feature_id)
        .ok_or_else(|| {
            format!(
                "mechanical pair {:?} has no feature {feature_id:?}",
                pair.id
            )
        })?;
    pair.features.remove(index);
    Ok(())
}

#[cfg(test)]
mod tests;
