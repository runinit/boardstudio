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

/// The pose whose forward map is [`super::pair::inverse`] for `mapping`.
///
/// The map's linear part is its own inverse, so the reverse direction is the same
/// reflection/rotation anchored at the negated image of the mapping translation.
/// It lets callers reuse forward-only helpers such as
/// [`super::pair::reflect_opening`] when deriving the second member back into the
/// first member's frame.
pub fn inverse_pose(mapping: Pose2) -> Pose2 {
    let fixed = super::pair::forward_vector(mapping, mapping.at);
    Pose2 {
        at: Vec2 {
            x: -fixed.x,
            y: -fixed.y,
        },
        rotation: mapping.rotation,
    }
}

/// Validates every mechanical mirror pair in the document.
///
/// Returns the first violation as a message naming the pair, instance or feature
/// it concerns. A document without hardware has no pairs and validates.
pub fn validate(document: &ProjectDoc) -> Result<(), String> {
    let Some(hardware) = document.hardware.as_ref() else {
        return Ok(());
    };
    let mut pair_ids: Vec<&str> = Vec::new();
    let mut owners: Vec<(&str, &str)> = Vec::new();
    for pair in &hardware.pairs {
        if pair.id.trim().is_empty() {
            return Err("a mechanical pair has an empty id".into());
        }
        if pair_ids.contains(&pair.id.as_str()) {
            return Err(format!("mechanical pair id {:?} is not unique", pair.id));
        }
        pair_ids.push(&pair.id);
        let [first, second] = &pair.members;
        if first == second {
            return Err(format!(
                "mechanical pair {:?} names the same physical instance twice",
                pair.id
            ));
        }
        for member in &pair.members {
            if !hardware
                .instances
                .iter()
                .any(|instance| instance.id == *member)
            {
                return Err(format!(
                    "mechanical pair {:?} names physical instance {member:?}, which is not in the document",
                    pair.id
                ));
            }
            if let Some((_, other)) = owners
                .iter()
                .find(|(instance, _)| instance == &member.as_str())
            {
                return Err(format!(
                    "physical instance {member:?} belongs to mechanical pairs {other:?} and {:?}",
                    pair.id
                ));
            }
            owners.push((member, &pair.id));
        }
        if !pair.mapping.at.x.is_finite()
            || !pair.mapping.at.y.is_finite()
            || !pair.mapping.rotation.is_finite()
        {
            return Err(format!(
                "mechanical pair {:?} has a non-finite mapping",
                pair.id
            ));
        }
        validate_features(pair, hardware)?;
    }
    Ok(())
}

fn validate_features(
    pair: &MechanicalMirrorPair,
    hardware: &HardwareConfiguration,
) -> Result<(), String> {
    let mut feature_ids: Vec<&str> = Vec::new();
    let mut member_feature_ids: Vec<(&str, &str)> = Vec::new();
    for feature in &pair.features {
        if feature.id.trim().is_empty() {
            return Err(format!(
                "mechanical pair {:?} has a feature with an empty id",
                pair.id
            ));
        }
        if feature_ids.contains(&feature.id.as_str()) {
            return Err(format!(
                "mechanical pair {:?} has more than one feature {:?}",
                pair.id, feature.id
            ));
        }
        feature_ids.push(&feature.id);
        if !value_matches_kind(feature.kind, &feature.value) {
            return Err(format!(
                "mechanical feature {:?} of pair {:?} has kind {:?} but a mismatched value",
                feature.id, pair.id, feature.kind
            ));
        }
        if !pair.members.contains(&feature.author) {
            return Err(format!(
                "mechanical feature {:?} of pair {:?} authors from {:?}, which is not a pair member",
                feature.id, pair.id, feature.author
            ));
        }
        validate_feature_value(&feature.id, &feature.value)?;
        if feature.members.len() != 2 {
            return Err(format!(
                "mechanical feature {:?} of pair {:?} must have one record per pair member, found {}",
                feature.id,
                pair.id,
                feature.members.len()
            ));
        }
        for member in &feature.members {
            if !pair.members.contains(&member.instance_id) {
                return Err(format!(
                    "mechanical feature {:?} of pair {:?} records {:?}, which is not a pair member",
                    feature.id, pair.id, member.instance_id
                ));
            }
            if member.feature_id.trim().is_empty() {
                return Err(format!(
                    "mechanical feature {:?} of pair {:?} has an empty member feature id for {:?}",
                    feature.id, pair.id, member.instance_id
                ));
            }
            if member_feature_ids.iter().any(|(instance, id)| {
                *instance == member.instance_id.as_str() && *id == member.feature_id.as_str()
            }) {
                return Err(format!(
                    "member feature id {:?} is not unique for physical instance {:?} in pair {:?}",
                    member.feature_id, member.instance_id, pair.id
                ));
            }
            member_feature_ids.push((&member.instance_id, &member.feature_id));
        }
        for member in &pair.members {
            let count = feature
                .members
                .iter()
                .filter(|record| &record.instance_id == member)
                .count();
            if count != 1 {
                return Err(format!(
                    "mechanical feature {:?} of pair {:?} must have exactly one record for {member:?}",
                    feature.id, pair.id
                ));
            }
        }
        let author = feature
            .members
            .iter()
            .find(|record| record.instance_id == feature.author)
            .ok_or_else(|| {
                format!(
                    "mechanical feature {:?} of pair {:?} has no record for its author {:?}",
                    feature.id, pair.id, feature.author
                )
            })?;
        if author.state != MechanicalFeatureState::Linked {
            return Err(format!(
                "mechanical feature {:?} of pair {:?} is authored by {:?} but that record is {:?}; the stored value is the linked value",
                feature.id, pair.id, feature.author, author.state
            ));
        }
    }
    validate_construction(pair, hardware)
}

fn validate_construction(
    pair: &MechanicalMirrorPair,
    hardware: &HardwareConfiguration,
) -> Result<(), String> {
    for member_id in &pair.members {
        let instance = hardware
            .instances
            .iter()
            .find(|instance| &instance.id == member_id)
            .ok_or_else(|| {
                format!(
                    "mechanical pair {:?} names physical instance {member_id:?}, which is not in the document",
                    pair.id
                )
            })?;
        match instance.construction_ownership {
            MechanicalConstructionOwnership::Pair => {
                if instance.mechanical.is_none() {
                    return Err(format!(
                        "physical instance {member_id:?} of mechanical pair {:?} is pair-owned but has no stored mechanical configuration",
                        pair.id
                    ));
                }
            }
            MechanicalConstructionOwnership::Local => {
                let Some(configuration) = instance.mechanical.as_ref() else {
                    return Err(format!(
                        "physical instance {member_id:?} of mechanical pair {:?} owns its construction locally but has no stored mechanical configuration",
                        pair.id
                    ));
                };
                if !construction_matches(configuration, &pair.construction) {
                    return Err(format!(
                        "physical instance {member_id:?} owns its construction locally, but its shared subset differs from mechanical pair {:?}'s construction",
                        pair.id
                    ));
                }
            }
            MechanicalConstructionOwnership::Legacy => {}
        }
    }
    Ok(())
}

fn construction_matches(
    configuration: &MechanicalConfiguration,
    construction: &MechanicalConstruction,
) -> bool {
    let mut expected = construction.clone();
    if let Some(layout) = expected.gasket_layout.as_mut() {
        layout.supports.clear();
    }
    shared_construction(configuration) == expected
}

fn value_matches_kind(kind: MechanicalFeatureKind, value: &MechanicalFeatureValue) -> bool {
    matches!(
        (kind, value),
        (
            MechanicalFeatureKind::Mount | MechanicalFeatureKind::ClosureMount,
            MechanicalFeatureValue::Mount { .. }
        ) | (
            MechanicalFeatureKind::Opening,
            MechanicalFeatureValue::Opening { .. }
        ) | (
            MechanicalFeatureKind::GasketSupport,
            MechanicalFeatureValue::GasketSupport { .. }
        )
    )
}

fn validate_feature_value(feature_id: &str, value: &MechanicalFeatureValue) -> Result<(), String> {
    let finite = |label: &str, value: f64| {
        if value.is_finite() {
            Ok(())
        } else {
            Err(format!(
                "mechanical feature {feature_id:?} has a non-finite {label}"
            ))
        }
    };
    match value {
        MechanicalFeatureValue::Mount { mount } => {
            finite("mount point", mount.at.x)?;
            finite("mount point", mount.at.y)?;
            finite("mount hole diameter", mount.hole_diameter)?;
            if let Some(diameter) = mount.boss_diameter {
                finite("mount boss diameter", diameter)?;
            }
            if let Some(height) = mount.height {
                finite("mount height", height)?;
            }
        }
        MechanicalFeatureValue::Opening { opening } => {
            finite("opening z", opening.z)?;
            finite("opening height", opening.height)?;
            for vertex in &opening.vertices {
                finite("opening vertex", vertex.point.x)?;
                finite("opening vertex", vertex.point.y)?;
            }
        }
        MechanicalFeatureValue::GasketSupport { anchor } => {
            finite("gasket anchor", anchor.anchor)?;
            if let Some(length) = anchor.length {
                finite("gasket support length", length)?;
            }
            if let Some(width) = anchor.width {
                finite("gasket support width", width)?;
            }
        }
    }
    Ok(())
}

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

fn feature_member<'a>(
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
mod tests {
    use super::*;

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

    fn instance(
        id: &str,
        ownership: MechanicalConstructionOwnership,
        mechanical: Option<MechanicalConfiguration>,
    ) -> PhysicalBoardInstance {
        PhysicalBoardInstance {
            id: id.into(),
            name: id.into(),
            board_id: "board".into(),
            half: "left".into(),
            role: "controller".into(),
            flipped: false,
            controller_part_id: None,
            mechanical,
            construction_ownership: ownership,
        }
    }

    fn local_instance(id: &str) -> PhysicalBoardInstance {
        instance(
            id,
            MechanicalConstructionOwnership::Local,
            Some(configuration("board")),
        )
    }

    fn doc(instances: Vec<PhysicalBoardInstance>, pairs: Vec<MechanicalMirrorPair>) -> ProjectDoc {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board("board", 1.6));
        document.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport: HardwareTransport::None,
            instances,
            boards: vec![],
            shared_construction: None,
            pairs,
        });
        document
    }

    fn pose(at_x: f64, at_y: f64, rotation: f64) -> Pose2 {
        Pose2 {
            at: Vec2 { x: at_x, y: at_y },
            rotation,
        }
    }

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

    fn vertex(id: &str, x: f64, y: f64) -> MechanicalOpeningVertex {
        MechanicalOpeningVertex {
            id: id.into(),
            point: Vec2 { x, y },
        }
    }

    fn opening(id: &str) -> MechanicalOpening {
        MechanicalOpening {
            id: id.into(),
            vertices: vec![
                vertex("v0", 0.0, 0.0),
                vertex("v1", 3.0, 0.0),
                vertex("v2", 3.0, 1.0),
                vertex("v3", 1.0, 2.5),
            ],
            z: 1.25,
            height: 3.5,
            next_vertex_identity: 0,
        }
    }

    fn anchor(id: &str) -> MechanicalGasketAnchor {
        MechanicalGasketAnchor {
            length: Some(12.0),
            width: Some(3.0),
            placement: Some(GasketPlacement::User),
            id: id.into(),
            region_id: "region".into(),
            outline_key: "outline".into(),
            anchor: 0.25,
            unlinked: false,
        }
    }

    fn feature(
        id: &str,
        kind: MechanicalFeatureKind,
        author: &str,
        value: MechanicalFeatureValue,
        members: &[(&str, &str, MechanicalFeatureState)],
    ) -> MechanicalFeature {
        MechanicalFeature {
            id: id.into(),
            kind,
            author: author.into(),
            value,
            members: members
                .iter()
                .map(|(instance_id, feature_id, state)| MechanicalFeatureMember {
                    instance_id: (*instance_id).into(),
                    feature_id: (*feature_id).into(),
                    state: *state,
                })
                .collect(),
        }
    }

    fn linked_mount_feature(id: &str, author: &str, mount_id: &str, at: Vec2) -> MechanicalFeature {
        feature(
            id,
            MechanicalFeatureKind::Mount,
            author,
            MechanicalFeatureValue::Mount {
                mount: mount(mount_id, at.x, at.y),
            },
            &[
                ("left", mount_id, MechanicalFeatureState::Linked),
                ("right", mount_id, MechanicalFeatureState::Linked),
            ],
        )
    }

    fn mirror_pair(
        id: &str,
        members: [&str; 2],
        mapping: Pose2,
        construction: MechanicalConstruction,
        features: Vec<MechanicalFeature>,
    ) -> MechanicalMirrorPair {
        MechanicalMirrorPair {
            id: id.into(),
            members: [members[0].into(), members[1].into()],
            mapping,
            construction,
            features,
        }
    }

    fn consistent_pair(features: Vec<MechanicalFeature>) -> MechanicalMirrorPair {
        mirror_pair(
            "pair-left-right",
            ["left", "right"],
            pose(0.0, 0.0, 0.0),
            shared_construction(&configuration("board")),
            features,
        )
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_point(actual: Vec2, expected: Vec2) {
        assert_close(actual.x, expected.x);
        assert_close(actual.y, expected.y);
    }

    fn assert_same_direction(actual: f64, expected: f64) {
        let difference = (actual - expected).rem_euclid(180.0);
        assert!(
            difference < 1e-9 || (180.0 - difference) < 1e-9,
            "expected direction {expected}, got {actual}"
        );
    }

    fn assert_same_opening(actual: &MechanicalOpening, expected: &MechanicalOpening) {
        assert_eq!(actual.id, expected.id);
        assert_eq!(actual.z, expected.z);
        assert_eq!(actual.height, expected.height);
        assert_eq!(actual.vertices.len(), expected.vertices.len());
        for (actual, expected) in actual.vertices.iter().zip(expected.vertices.iter()) {
            assert_eq!(actual.id, expected.id);
            assert_point(actual.point, expected.point);
        }
    }

    fn gasket_layout(supports: Vec<MechanicalGasketAnchor>) -> MechanicalGasketLayout {
        MechanicalGasketLayout {
            auto_size: Some(true),
            adhesive_thickness: Some(0.1),
            minimum_foam_thickness: Some(0.5),
            preset_id: None,
            material: Some("Poron".into()),
            length: 99.0,
            width: 3.0,
            thickness: 2.0,
            compression: 0.15,
            supports,
        }
    }

    fn battery() -> MechanicalBattery {
        serde_json::from_value(serde_json::json!({
            "size": { "x": 30.0, "y": 20.0, "z": 6.0 },
            "at": { "x": 1.0, "y": 2.0 },
            "cableExit": { "x": 3.0, "y": 2.0 }
        }))
        .unwrap()
    }

    fn profile(id: &str) -> MechanicalPartProfile {
        serde_json::from_value(serde_json::json!({
            "definitionId": id, "source": "test", "cutouts": [], "plateToPcb": 3.5
        }))
        .unwrap()
    }

    fn stabilizer() -> MechanicalStabilizerOverride {
        serde_json::from_value(serde_json::json!({
            "partId": "part", "kind": "pcb-mount", "units": 2.0
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

    fn part_process(id: &str) -> MechanicalPartProcess {
        serde_json::from_value(serde_json::json!({
            "partId": id, "method": "cnc", "material": "Aluminium", "thickness": 1.5
        }))
        .unwrap()
    }

    #[test]
    fn validate_accepts_a_consistent_pair() {
        let document = doc(
            vec![local_instance("left"), local_instance("right")],
            vec![consistent_pair(vec![linked_mount_feature(
                "feature-1",
                "left",
                "mount-1",
                Vec2 { x: 2.0, y: 3.0 },
            )])],
        );
        validate(&document).unwrap();
    }

    #[test]
    fn validate_accepts_a_document_without_hardware() {
        validate(&ProjectDoc::empty("doc", "Fixture")).unwrap();
    }

    #[test]
    fn validate_rejects_duplicate_members() {
        let mut pair = consistent_pair(vec![]);
        pair.members = ["left".into(), "left".into()];
        let document = doc(vec![local_instance("left")], vec![pair]);
        let error = validate(&document).unwrap_err();
        assert!(error.contains("same physical instance twice"), "{error}");
    }

    #[test]
    fn validate_rejects_an_unknown_member() {
        let mut pair = consistent_pair(vec![]);
        pair.members = ["left".into(), "ghost".into()];
        let document = doc(vec![local_instance("left")], vec![pair]);
        let error = validate(&document).unwrap_err();
        assert!(error.contains("not in the document"), "{error}");
    }

    #[test]
    fn validate_rejects_a_third_instance_joining_a_pair_and_an_instance_in_two_pairs() {
        let second = mirror_pair(
            "pair-right-third",
            ["right", "third"],
            pose(0.0, 0.0, 0.0),
            shared_construction(&configuration("board")),
            vec![],
        );
        let document = doc(
            vec![
                local_instance("left"),
                local_instance("right"),
                local_instance("third"),
            ],
            vec![consistent_pair(vec![]), second],
        );
        let error = validate(&document).unwrap_err();
        assert!(error.contains("belongs to mechanical pairs"), "{error}");
        assert!(error.contains("\"right\""), "{error}");
    }

    #[test]
    fn validate_rejects_a_non_finite_mapping() {
        for mapping in [
            pose(f64::NAN, 0.0, 0.0),
            pose(0.0, f64::INFINITY, 0.0),
            pose(0.0, 0.0, f64::NEG_INFINITY),
        ] {
            let document = doc(
                vec![local_instance("left"), local_instance("right")],
                vec![mirror_pair(
                    "pair-left-right",
                    ["left", "right"],
                    mapping,
                    shared_construction(&configuration("board")),
                    vec![],
                )],
            );
            let error = validate(&document).unwrap_err();
            assert!(error.contains("non-finite mapping"), "{error}");
        }
    }

    #[test]
    fn validate_rejects_a_non_finite_authored_value() {
        let mut feature = linked_mount_feature("feature-1", "left", "mount-1", Vec2::default());
        if let MechanicalFeatureValue::Mount { mount } = &mut feature.value {
            mount.at.x = f64::NAN;
        }
        let document = doc(
            vec![local_instance("left"), local_instance("right")],
            vec![consistent_pair(vec![feature])],
        );
        let error = validate(&document).unwrap_err();
        assert!(error.contains("non-finite mount point"), "{error}");
    }

    #[test]
    fn validate_rejects_a_mismatched_kind_and_value() {
        let feature = feature(
            "feature-1",
            MechanicalFeatureKind::Opening,
            "left",
            MechanicalFeatureValue::Mount {
                mount: mount("mount-1", 2.0, 3.0),
            },
            &[
                ("left", "mount-1", MechanicalFeatureState::Linked),
                ("right", "mount-1", MechanicalFeatureState::Linked),
            ],
        );
        let document = doc(
            vec![local_instance("left"), local_instance("right")],
            vec![consistent_pair(vec![feature])],
        );
        let error = validate(&document).unwrap_err();
        assert!(error.contains("mismatched value"), "{error}");
    }

    #[test]
    fn validate_rejects_an_unknown_author() {
        let mut feature = linked_mount_feature("feature-1", "left", "mount-1", Vec2::default());
        feature.author = "ghost".into();
        let document = doc(
            vec![local_instance("left"), local_instance("right")],
            vec![consistent_pair(vec![feature])],
        );
        let error = validate(&document).unwrap_err();
        assert!(error.contains("not a pair member"), "{error}");
    }

    #[test]
    fn validate_rejects_a_missing_member_record() {
        let mut feature = linked_mount_feature("feature-1", "left", "mount-1", Vec2::default());
        feature.members.pop();
        let document = doc(
            vec![local_instance("left"), local_instance("right")],
            vec![consistent_pair(vec![feature])],
        );
        let error = validate(&document).unwrap_err();
        assert!(error.contains("one record per pair member"), "{error}");
    }

    #[test]
    fn validate_rejects_a_foreign_member_record() {
        let mut feature = linked_mount_feature("feature-1", "left", "mount-1", Vec2::default());
        feature.members[1].instance_id = "third".into();
        let document = doc(
            vec![local_instance("left"), local_instance("right")],
            vec![consistent_pair(vec![feature])],
        );
        let error = validate(&document).unwrap_err();
        assert!(error.contains("not a pair member"), "{error}");
    }

    #[test]
    fn validate_rejects_an_authoring_member_that_is_not_linked() {
        let mut feature = linked_mount_feature("feature-1", "left", "mount-1", Vec2::default());
        feature.members[0].state = MechanicalFeatureState::Omitted;
        let document = doc(
            vec![local_instance("left"), local_instance("right")],
            vec![consistent_pair(vec![feature])],
        );
        let error = validate(&document).unwrap_err();
        assert!(
            error.contains("the stored value is the linked value"),
            "{error}"
        );
    }

    #[test]
    fn validate_rejects_duplicate_feature_and_member_feature_ids() {
        let first = linked_mount_feature("feature-1", "left", "mount-1", Vec2::default());
        let mut second = linked_mount_feature("feature-1", "right", "mount-2", Vec2::default());
        second.members[0].feature_id = "other".into();
        second.members[1].feature_id = "other".into();
        let document = doc(
            vec![local_instance("left"), local_instance("right")],
            vec![consistent_pair(vec![first, second])],
        );
        let error = validate(&document).unwrap_err();
        assert!(error.contains("more than one feature"), "{error}");

        let first = linked_mount_feature("feature-1", "left", "mount-1", Vec2::default());
        let second = linked_mount_feature("feature-2", "left", "mount-1", Vec2::default());
        let document = doc(
            vec![local_instance("left"), local_instance("right")],
            vec![consistent_pair(vec![first, second])],
        );
        let error = validate(&document).unwrap_err();
        assert!(
            error.contains("not unique for physical instance"),
            "{error}"
        );
    }

    #[test]
    fn validate_enforces_construction_consistency_for_locally_owned_members() {
        // A locally owned member must already store the pair's shared subset.
        let mut member = configuration("board");
        member.plate_thickness = 9.0;
        let document = doc(
            vec![
                instance("left", MechanicalConstructionOwnership::Local, Some(member)),
                local_instance("right"),
            ],
            vec![consistent_pair(vec![])],
        );
        let error = validate(&document).unwrap_err();
        assert!(error.contains("shared subset differs"), "{error}");

        // A pair-owned member takes the pair construction as authority, so its
        // stored shared subset is only an overlay base.
        let mut member = configuration("board");
        member.plate_thickness = 9.0;
        let document = doc(
            vec![
                instance("left", MechanicalConstructionOwnership::Pair, Some(member)),
                local_instance("right"),
            ],
            vec![consistent_pair(vec![])],
        );
        validate(&document).unwrap();

        // A pair-owned member still needs a stored configuration for its local fields.
        let document = doc(
            vec![
                instance("left", MechanicalConstructionOwnership::Pair, None),
                local_instance("right"),
            ],
            vec![consistent_pair(vec![])],
        );
        let error = validate(&document).unwrap_err();
        assert!(error.contains("pair-owned but has no stored"), "{error}");
    }

    #[test]
    fn linked_mount_derives_the_reflected_counterpart_at_the_mapped_point() {
        let mapping = pose(10.0, 5.0, 90.0);
        let feature = linked_mount_feature("feature-1", "left", "mount-1", Vec2 { x: 2.0, y: 3.0 });
        let pair = mirror_pair(
            "pair-left-right",
            ["left", "right"],
            mapping,
            shared_construction(&configuration("board")),
            vec![feature.clone()],
        );

        let derived = counterpart(&pair, &feature, "left").unwrap().unwrap();
        assert_eq!(derived.instance_id, "right");
        assert_eq!(derived.feature_id, "mount-1");
        let MechanicalFeatureValue::Mount { mount: derived } = &derived.value else {
            panic!("a linked mount must derive a mount");
        };
        // `q = R(90) * (-2, 3) + (10, 5) = (-3, -2) + (10, 5)`.
        assert_point(derived.at, Vec2 { x: 7.0, y: 3.0 });
        assert_eq!(derived.id, "mount-1");
        assert_eq!(derived.kind, MountKind::Hole);
        assert_close(derived.hole_diameter, 2.2);

        // The orientation rule is `rotation - angle`: the direction the mount
        // carries about the map origin transforms that way (directions are modulo
        // 180 degrees under a reflection).
        let authored_direction = 3.0_f64.atan2(2.0).to_degrees();
        let derived_direction = (derived.at.y - mapping.at.y).atan2(derived.at.x - mapping.at.x);
        assert_same_direction(
            derived_direction.to_degrees(),
            super::super::pair::forward_angle(mapping, authored_direction),
        );
    }

    #[test]
    fn asymmetric_opening_keeps_vertex_ids_reverses_winding_and_preserves_z_and_height() {
        let mapping = pose(0.0, 0.0, 0.0);
        let source = opening("opening-1");
        let feature = feature(
            "feature-1",
            MechanicalFeatureKind::Opening,
            "left",
            MechanicalFeatureValue::Opening {
                opening: source.clone(),
            },
            &[
                ("left", "opening-1", MechanicalFeatureState::Linked),
                ("right", "opening-1", MechanicalFeatureState::Linked),
            ],
        );
        let pair = mirror_pair(
            "pair-left-right",
            ["left", "right"],
            mapping,
            shared_construction(&configuration("board")),
            vec![feature.clone()],
        );

        let derived = counterpart(&pair, &feature, "left").unwrap().unwrap();
        let MechanicalFeatureValue::Opening { opening: derived } = &derived.value else {
            panic!("a linked opening must derive an opening");
        };
        let ids: Vec<&str> = derived
            .vertices
            .iter()
            .map(|vertex| vertex.id.as_str())
            .collect();
        assert_eq!(ids, ["v3", "v2", "v1", "v0"]);
        assert_eq!(derived.z, source.z);
        assert_eq!(derived.height, source.height);
        assert_point(derived.vertices[0].point, Vec2 { x: -1.0, y: 2.5 });
        assert_ne!(derived.points(), source.points());
        assert_same_opening(
            derived,
            &super::super::pair::reflect_opening(mapping, &source),
        );
    }

    #[test]
    fn inverse_pose_matches_the_exact_inverse_map() {
        for mapping in [
            pose(12.5, -4.25, 0.0),
            pose(-3.25, 7.5, 37.5),
            pose(10.0, 5.0, 90.0),
        ] {
            let reverse = inverse_pose(mapping);
            for point in [
                Vec2 { x: 0.5, y: -1.25 },
                Vec2 { x: 3.0, y: 2.75 },
                Vec2 { x: -2.25, y: 4.5 },
            ] {
                assert_point(
                    super::super::pair::forward(reverse, point),
                    super::super::pair::inverse(mapping, point),
                );
                assert_point(
                    super::super::pair::forward(
                        mapping,
                        super::super::pair::forward(reverse, point),
                    ),
                    point,
                );
            }
        }
    }

    #[test]
    fn forward_and_inverse_maps_round_trip_an_asymmetric_feature() {
        // Authoring on the second member derives through the exact inverse map.
        let mapping = pose(-4.5, 2.25, 37.5);
        let source = opening("opening-1");
        let feature = feature(
            "feature-1",
            MechanicalFeatureKind::Opening,
            "right",
            MechanicalFeatureValue::Opening {
                opening: source.clone(),
            },
            &[
                ("left", "opening-1", MechanicalFeatureState::Linked),
                ("right", "opening-1", MechanicalFeatureState::Linked),
            ],
        );
        let pair = mirror_pair(
            "pair-left-right",
            ["left", "right"],
            mapping,
            shared_construction(&configuration("board")),
            vec![feature.clone()],
        );

        let derived = counterpart(&pair, &feature, "right").unwrap().unwrap();
        assert_eq!(derived.instance_id, "left");
        let MechanicalFeatureValue::Opening { opening: derived } = &derived.value else {
            panic!("a linked opening must derive an opening");
        };
        assert_same_opening(
            derived,
            &super::super::pair::reflect_opening(inverse_pose(mapping), &source),
        );
        // An authoring half on the second member must use the inverse map; an
        // asymmetric fixture keeps the forward map from hiding here.
        assert_ne!(
            derived.points(),
            super::super::pair::reflect_opening(mapping, &source).points()
        );
        // Mapping the derived geometry back returns the authored opening, order
        // included, so a second reflection cannot hide in asymmetric geometry.
        assert_same_opening(
            &super::super::pair::reflect_opening(mapping, derived),
            &source,
        );

        // The authoring half's own derived is the stored value, unmapped.
        let back = counterpart(&pair, &feature, "left").unwrap().unwrap();
        assert_eq!(back.instance_id, "right");
        assert_same_opening(&source, &opening_value(&back.value));

        // The same round trip holds for an asymmetric mount.
        let mount_feature =
            linked_mount_feature("feature-2", "right", "mount-2", Vec2 { x: 1.5, y: -2.25 });
        let pair = mirror_pair(
            "pair-left-right",
            ["left", "right"],
            mapping,
            shared_construction(&configuration("board")),
            vec![mount_feature.clone()],
        );
        let derived = counterpart(&pair, &mount_feature, "right")
            .unwrap()
            .unwrap();
        let MechanicalFeatureValue::Mount { mount: derived } = &derived.value else {
            panic!("a linked mount must derive a mount");
        };
        assert_point(
            super::super::pair::forward(mapping, derived.at),
            Vec2 { x: 1.5, y: -2.25 },
        );
        assert_ne!(
            derived.at,
            super::super::pair::forward(mapping, Vec2 { x: 1.5, y: -2.25 })
        );
    }

    fn opening_value(value: &MechanicalFeatureValue) -> MechanicalOpening {
        match value {
            MechanicalFeatureValue::Opening { opening } => opening.clone(),
            other => panic!("expected an opening value, got {other:?}"),
        }
    }

    #[test]
    fn same_pcb_opposite_flip_and_distinct_pcb_mappings_reflect_the_counterpart() {
        // One reused PCB with opposite physical flips: the mapping is the physical
        // reflection about the halves' shared vertical axis.
        let reused = mirror_pair(
            "pair-reused",
            ["left", "right"],
            pose(80.0, 0.0, 0.0),
            shared_construction(&configuration("board")),
            vec![],
        );
        let feature =
            linked_mount_feature("feature-1", "left", "mount-1", Vec2 { x: 10.0, y: 5.0 });
        let derived = counterpart(&reused, &feature, "left").unwrap().unwrap();
        let MechanicalFeatureValue::Mount { mount: derived } = &derived.value else {
            panic!("a linked mount must derive a mount");
        };
        assert_point(derived.at, Vec2 { x: 70.0, y: 5.0 });

        // Distinct PCBs carry a declared translation and rotation.
        let distinct = mirror_pair(
            "pair-distinct",
            ["left", "right"],
            pose(120.0, -5.0, 90.0),
            shared_construction(&configuration("board")),
            vec![],
        );
        let derived = counterpart(&distinct, &feature, "left").unwrap().unwrap();
        let MechanicalFeatureValue::Mount { mount: derived } = &derived.value else {
            panic!("a linked mount must derive a mount");
        };
        // `q = R(90) * (-10, 5) + (120, -5) = (-5, -10) + (120, -5)`.
        assert_point(derived.at, Vec2 { x: 115.0, y: -15.0 });
        assert_ne!(derived.at, Vec2 { x: 10.0, y: 5.0 });
    }

    #[test]
    fn local_feature_derives_no_counterpart_while_linked_features_still_derive_theirs() {
        let linked = linked_mount_feature("feature-1", "right", "mount-1", Vec2 { x: 2.0, y: 3.0 });
        let local = feature(
            "feature-2",
            MechanicalFeatureKind::Mount,
            "right",
            MechanicalFeatureValue::Mount {
                mount: mount("mount-2", 4.0, 5.0),
            },
            &[
                ("left", "mount-2", MechanicalFeatureState::Local),
                ("right", "mount-2", MechanicalFeatureState::Linked),
            ],
        );
        let pair = consistent_pair(vec![linked.clone(), local.clone()]);

        assert!(counterpart(&pair, &local, "left").unwrap().is_none());
        assert!(counterpart(&pair, &local, "right").unwrap().is_none());
        let derived = counterpart(&pair, &linked, "right").unwrap().unwrap();
        assert_eq!(derived.instance_id, "left");
        assert_eq!(derived.feature_id, "mount-1");
    }

    #[test]
    fn omitted_member_derives_nothing_but_keeps_the_record() {
        let feature = feature(
            "feature-1",
            MechanicalFeatureKind::Mount,
            "right",
            MechanicalFeatureValue::Mount {
                mount: mount("mount-1", 2.0, 3.0),
            },
            &[
                ("left", "mount-1", MechanicalFeatureState::Omitted),
                ("right", "mount-1", MechanicalFeatureState::Linked),
            ],
        );
        let pair = consistent_pair(vec![feature.clone()]);

        assert!(counterpart(&pair, &feature, "left").unwrap().is_none());
        assert!(counterpart(&pair, &feature, "right").unwrap().is_none());
        assert_eq!(
            member_state(&pair, "left", "mount-1"),
            Some(MechanicalFeatureState::Omitted)
        );
        assert_eq!(
            member_state(&pair, "right", "mount-1"),
            Some(MechanicalFeatureState::Linked)
        );
    }

    #[test]
    fn counterpart_rejects_instances_that_are_not_pair_members() {
        let feature = linked_mount_feature("feature-1", "left", "mount-1", Vec2::default());
        let pair = consistent_pair(vec![feature.clone()]);
        let error = counterpart(&pair, &feature, "ghost").unwrap_err();
        assert!(error.contains("not a member of mechanical pair"), "{error}");
    }

    #[test]
    fn mirror_support_mirrors_resolved_geometry_tangent_and_normal() {
        let mapping = pose(10.0, 5.0, 90.0);
        let support = MechanicalGasketSupport {
            fit_error: None,
            placement: Some(GasketPlacement::User),
            id: "support-1".into(),
            region_id: "region".into(),
            outline_key: "outline".into(),
            anchor: 0.25,
            at: Vec2 { x: 2.0, y: 3.0 },
            tangent: Vec2 { x: 1.0, y: 0.0 },
            normal: Vec2 { x: 0.0, y: 1.0 },
            length: 12.0,
            width: 3.0,
            z: 1.0,
            thickness: 0.5,
            pair_id: Some("pair-left-right".into()),
            mirror_axis: None,
            unlinked: false,
        };
        let mirrored = mirror_support(mapping, &support);
        assert_point(mirrored.at, Vec2 { x: 7.0, y: 3.0 });
        assert_point(mirrored.tangent, Vec2 { x: 0.0, y: -1.0 });
        assert_point(mirrored.normal, Vec2 { x: -1.0, y: 0.0 });
        assert_close(mirrored.length, support.length);
        assert_close(mirrored.width, support.width);
        assert_close(mirrored.z, support.z);
        assert_close(mirrored.thickness, support.thickness);
        assert_eq!(mirrored.id, support.id);
        assert_eq!(mirrored.region_id, support.region_id);
        assert_eq!(mirrored.anchor, support.anchor);
        assert_eq!(mirrored.placement, support.placement);

        // The mirrored frame stays perpendicular and the reverse pose restores it.
        let dot = mirrored.tangent.x * mirrored.normal.x + mirrored.tangent.y * mirrored.normal.y;
        assert_close(dot, 0.0);
        let restored = mirror_support(inverse_pose(mapping), &mirrored);
        assert_point(restored.at, support.at);
        assert_point(restored.tangent, support.tangent);
        assert_point(restored.normal, support.normal);
    }

    #[test]
    fn shared_construction_round_trips_the_shared_subset_and_leaves_local_fields_untouched() {
        let mut configuration = configuration("board");
        configuration.method = PlateMethod::Cnc;
        configuration.mount = MechanicalMount::Rigid;
        configuration.integrated_plate_frame = true;
        configuration.bottom_style = Some(MechanicalBottomStyle::Sheet);
        configuration.middle_frame = Some(true);
        configuration.plate_thickness = 7.0;
        configuration.plate_foam_thickness = 0.5;
        configuration.bottom_foam_thickness = 0.25;
        configuration.bottom_thickness = 3.0;
        configuration.plate_to_pcb = 4.0;
        configuration.wall_thickness = 2.5;
        configuration.clearance = 0.3;
        configuration.gasket = Some(Gasket {
            inset: 1.0,
            width: 2.0,
            depth: 3.0,
        });
        configuration.gasket_travel = Some(0.6);
        configuration.opening_allowance = Some(0.4);
        configuration.part_processes = Some(vec![part_process("plate")]);
        let mut internal_gasket = crate::mechanical::default_internal_gasket();
        internal_gasket.support_count = 7;
        configuration.internal_gasket = Some(internal_gasket);
        configuration.gasket_layout = Some(gasket_layout(vec![anchor("support-1")]));
        // Local fields.
        configuration.battery = Some(battery());
        configuration.battery_height = 6.0;
        configuration.mounts = vec![mount("mount-1", 1.0, 2.0)];
        configuration.closure_mounts = Some(vec![mount("closure-1", 3.0, 4.0)]);
        configuration.openings = Some(vec![opening("opening-1")]);
        configuration.stabilizers = Some(vec![stabilizer()]);
        configuration.profiles = vec![profile("plate")];
        configuration.hardware = Some(vec![hardware_specification("hardware-1")]);
        configuration.critical_fits = Some(vec![critical_fit("fit-1")]);

        let construction = shared_construction(&configuration);
        assert_eq!(construction.method, PlateMethod::Cnc);
        assert_eq!(construction.mount, MechanicalMount::Rigid);
        assert_eq!(construction.plate_thickness, 7.0);
        assert_eq!(construction.clearance, 0.3);
        assert_eq!(construction.gasket_travel, Some(0.6));
        assert_eq!(construction.opening_allowance, Some(0.4));
        assert_eq!(construction.part_processes.as_ref().unwrap().len(), 1);
        assert_eq!(
            construction.internal_gasket.as_ref().unwrap().support_count,
            7
        );
        // Gasket support anchors belong to one half, so they are not captured.
        assert!(
            construction
                .gasket_layout
                .as_ref()
                .unwrap()
                .supports
                .is_empty()
        );

        // Round-trip onto a configuration whose shared subset was cleared.
        let mut applied = configuration.clone();
        applied.method = PlateMethod::Printed;
        applied.mount = MechanicalMount::Tray;
        applied.integrated_plate_frame = false;
        applied.bottom_style = None;
        applied.middle_frame = None;
        applied.plate_thickness = 1.0;
        applied.plate_foam_thickness = 1.0;
        applied.bottom_foam_thickness = 1.0;
        applied.bottom_thickness = 1.0;
        applied.plate_to_pcb = 1.0;
        applied.wall_thickness = 1.0;
        applied.clearance = 1.0;
        applied.gasket = None;
        applied.gasket_travel = None;
        applied.opening_allowance = None;
        applied.part_processes = None;
        applied.internal_gasket = None;
        // The member keeps its own gasket layout, including its own supports.
        apply_construction(&mut applied, &construction);

        assert_eq!(shared_construction(&applied), construction);
        // Local fields, including the member's own supports, are untouched.
        assert_eq!(applied.board_id, configuration.board_id);
        assert_eq!(applied.pcb_thickness, configuration.pcb_thickness);
        assert_eq!(applied.battery, configuration.battery);
        assert_eq!(applied.battery_height, configuration.battery_height);
        assert_eq!(applied.mounts, configuration.mounts);
        assert_eq!(applied.closure_mounts, configuration.closure_mounts);
        assert_eq!(applied.openings, configuration.openings);
        assert_eq!(applied.stabilizers, configuration.stabilizers);
        assert_eq!(applied.profiles, configuration.profiles);
        assert_eq!(applied.hardware, configuration.hardware);
        assert_eq!(applied.critical_fits, configuration.critical_fits);
        assert_eq!(
            applied.gasket_layout.as_ref().unwrap().supports,
            vec![anchor("support-1")]
        );

        // A local member's own supports survive even when it had none before.
        let mut without_layout = configuration.clone();
        without_layout.gasket_layout = None;
        apply_construction(&mut without_layout, &construction);
        assert!(
            without_layout
                .gasket_layout
                .as_ref()
                .unwrap()
                .supports
                .is_empty()
        );
        assert_eq!(without_layout.battery, configuration.battery);
    }

    #[test]
    fn apply_construction_leaves_local_fields_untouched_on_a_different_member() {
        let source = configuration("board");
        let construction = shared_construction(&source);

        let mut member = configuration("board");
        member.board_id = "other-board".into();
        member.pcb_thickness = 2.0;
        member.mounts = vec![mount("member-mount", 9.0, 8.0)];
        member.openings = Some(vec![opening("member-opening")]);
        apply_construction(&mut member, &construction);

        assert_eq!(member.board_id, "other-board");
        assert_eq!(member.pcb_thickness, 2.0);
        assert_eq!(member.mounts, vec![mount("member-mount", 9.0, 8.0)]);
        assert_eq!(member.openings, Some(vec![opening("member-opening")]));
    }

    #[test]
    fn lifecycle_helpers_record_links_exceptions_and_linked_deletes() {
        let mut pair = consistent_pair(vec![]);
        let feature =
            linked_mount_feature("feature-1", "right", "mount-1", Vec2 { x: 2.0, y: 3.0 });
        add_linked_feature(&mut pair, feature).unwrap();

        // Both members are linked with the authored value's id.
        assert_eq!(
            member_state(&pair, "left", "mount-1"),
            Some(MechanicalFeatureState::Linked)
        );
        assert_eq!(
            member_state(&pair, "right", "mount-1"),
            Some(MechanicalFeatureState::Linked)
        );

        make_local(&mut pair, "left", "mount-1").unwrap();
        assert_eq!(
            member_state(&pair, "left", "mount-1"),
            Some(MechanicalFeatureState::Local)
        );
        rejoin_member(&mut pair, "left", "mount-1").unwrap();
        assert_eq!(
            member_state(&pair, "left", "mount-1"),
            Some(MechanicalFeatureState::Linked)
        );
        omit_member(&mut pair, "left", "mount-1").unwrap();
        assert_eq!(
            member_state(&pair, "left", "mount-1"),
            Some(MechanicalFeatureState::Omitted)
        );
        // An omitted member keeps its record, so the other half still derives.
        assert_eq!(pair.features[0].members.len(), 2);

        // Errors name the missing member feature and feature.
        assert!(
            set_member_state(
                &mut pair,
                "ghost",
                "mount-1",
                MechanicalFeatureState::Linked
            )
            .is_err()
        );
        assert!(make_local(&mut pair, "left", "ghost").is_err());
        assert!(remove_linked_feature(&mut pair, "ghost").is_err());

        remove_linked_feature(&mut pair, "feature-1").unwrap();
        assert!(pair.features.is_empty());
        assert_eq!(member_state(&pair, "left", "mount-1"), None);
    }

    #[test]
    fn add_linked_feature_rejects_unknown_authors_and_duplicate_ids() {
        let mut pair = consistent_pair(vec![]);
        let mut feature = linked_mount_feature("feature-1", "ghost", "mount-1", Vec2::default());
        assert!(add_linked_feature(&mut pair, feature.clone()).is_err());
        feature.author = "left".into();
        add_linked_feature(&mut pair, feature.clone()).unwrap();
        assert!(add_linked_feature(&mut pair, feature).is_err());

        let mut unnamed = linked_mount_feature("feature-2", "left", "", Vec2::default());
        if let MechanicalFeatureValue::Mount { mount } = &mut unnamed.value {
            mount.id = String::new();
        }
        assert!(add_linked_feature(&mut pair, unnamed).is_err());
    }
}
