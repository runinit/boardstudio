//! Full mechanical-configuration replacement through the existing ownership rules.
//!
//! A replacement is staged on a clone, then partitioned through `persist_edit` so pair-owned
//! construction and linked features remain pair-owned while local settings stay with the member.
//! This module stays private to `mechanical::edit`; the prepared mirrored transaction calls its
//! narrow parent adapter.

use super::*;
use crate::mechanical::change::MechanicalEdit;

/// Replaces one physical instance's effective configuration without publishing partial state.
pub(super) fn apply_instance_configuration(
    document: &mut ProjectDoc,
    board_contours: &[BoardContours],
    target: &MechanicalTarget,
    requested: Option<MechanicalConfiguration>,
) -> Result<Vec<String>, String> {
    let mut draft = document.clone();
    let changed = replace_instance_staged(&mut draft, board_contours, target, requested)?;
    *document = draft;
    Ok(changed)
}

fn replace_instance_staged(
    document: &mut ProjectDoc,
    board_contours: &[BoardContours],
    target: &MechanicalTarget,
    requested: Option<MechanicalConfiguration>,
) -> Result<Vec<String>, String> {
    let MechanicalTarget::Instance { instance_id } = target else {
        return Err("A targeted mechanical replacement requires a physical instance.".into());
    };
    let board_id = unique_instance_board_id(document, instance_id)?;
    features::validate(document)?;

    let Some(mut after) = requested else {
        let hardware = document
            .hardware
            .as_ref()
            .ok_or("Physical instances are unavailable")?;
        let paired = hardware
            .pairs
            .iter()
            .any(|pair| pair.members.iter().any(|member| member == instance_id));
        let has_configuration = hardware
            .instances
            .iter()
            .any(|instance| instance.id == *instance_id && instance.mechanical.is_some());
        if !has_configuration && !paired {
            return Ok(Vec::new());
        }
        let edit = MechanicalEdit {
            target: target.clone(),
            change: MechanicalChange::Disable,
            // Pair dissolution can rematerialize closure geometry on the survivor. A
            // single-stack clear only removes or retains already projected geometry.
            mounting_hole: paired
                .then(|| normalized_mounting_hole().map(Box::new))
                .transpose()?,
        };
        let changed = disable(document, target, &board_id, &edit)?;
        features::validate(document)?;
        return Ok(changed);
    };

    if after.board_id != board_id {
        return Err(format!(
            "Mechanical configuration board {:?} does not match physical instance {instance_id:?} on board {board_id:?}.",
            after.board_id
        ));
    }
    validate_unambiguous_feature_ids(&after, "replacement")?;
    validate_mounts(&after.mounts)?;
    validate_mounts(after.closure_mounts.as_deref().unwrap_or_default())?;
    if let Some(openings) = after.openings.as_deref() {
        validate_openings(openings)?;
    }
    if let Some(layout) = &after.gasket_layout {
        validate_gasket_support_anchors(&layout.supports)?;
        for support in &layout.supports {
            if let Some(length) = support.length {
                validate_dimension(MechanicalDimension::GasketSupportLength, length)?;
            }
            if let Some(width) = support.width {
                validate_dimension(MechanicalDimension::GasketSupportWidth, width)?;
            }
        }
    }

    let disabled_unpaired = is_disabled_unpaired_instance(document, instance_id)?;
    let before = if disabled_unpaired {
        // A supplied effective configuration is the enable value for an instance
        // that has no stored or inherited configuration yet. Using it as the
        // comparison baseline avoids manufacturing defaults that the request did
        // not provide; projection below still runs for the newly enabled stack.
        after.clone()
    } else {
        effective_inputs(document, board_contours, target)?.configuration
    };
    validate_unambiguous_feature_ids(&before, "accepted")?;
    preserve_identity_reservations(document, target, &before, &mut after);
    reject_omitted_reintroduction(document, target, &before, &after)?;

    let projection_required = disabled_unpaired || replacement_affects_projection(&before, &after);
    let mut changed = vec![owner_id(target, &board_id)];
    persist_edit(document, target, &before, after, &mut changed)?;

    if projection_required {
        let template = normalized_mounting_hole()?;
        project(document, Some(&template), &mut changed)?;
    }
    features::validate(document)?;
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Whether this exact, unpaired instance has no effective configuration to replace.
///
/// Only Legacy instances can inherit an absent local slot, from the shared source
/// used by `effective_inputs`; preserve that route whenever such a source exists so
/// malformed inherited state is still reported by the resolver.
fn is_disabled_unpaired_instance(document: &ProjectDoc, instance_id: &str) -> Result<bool, String> {
    let hardware = document
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?;
    let mut matches = hardware
        .instances
        .iter()
        .filter(|instance| instance.id == instance_id);
    let instance = matches
        .next()
        .ok_or_else(|| format!("Physical instance {instance_id:?} is unavailable."))?;
    if matches.next().is_some() {
        return Err(format!("Physical instance {instance_id:?} is ambiguous."));
    }
    if !document
        .boards
        .iter()
        .any(|board| board.id == instance.board_id)
    {
        return Err(format!(
            "physical instance board {:?} is not in the document",
            instance.board_id
        ));
    }
    if instance.mechanical.is_some()
        || hardware
            .pairs
            .iter()
            .any(|pair| pair.members.iter().any(|member| member == instance_id))
    {
        return Ok(false);
    }

    let has_legacy_source = hardware.shared_construction.is_some()
        || hardware
            .instances
            .iter()
            .any(|candidate| candidate.mechanical.is_some());
    Ok(
        instance.construction_ownership != MechanicalConstructionOwnership::Legacy
            || !has_legacy_source,
    )
}

fn unique_instance_board_id(document: &ProjectDoc, instance_id: &str) -> Result<String, String> {
    let hardware = document
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?;
    let mut matches = hardware
        .instances
        .iter()
        .filter(|instance| instance.id == instance_id);
    let instance = matches
        .next()
        .ok_or_else(|| format!("Physical instance {instance_id:?} is unavailable."))?;
    if matches.next().is_some() {
        return Err(format!("Physical instance {instance_id:?} is ambiguous."));
    }
    Ok(instance.board_id.clone())
}

fn preserve_identity_reservations(
    document: &ProjectDoc,
    target: &MechanicalTarget,
    before: &MechanicalConfiguration,
    after: &mut MechanicalConfiguration,
) {
    let accepted = identity_reservation_of(before);
    let requested = identity_reservation_of(after);
    let reserved = reserved_identity_floor(document, target);
    let floor = (
        reserved
            .0
            .max(accepted.next_opening_identity.saturating_sub(1))
            .max(requested.next_opening_identity.saturating_sub(1)),
        reserved
            .1
            .max(accepted.next_vertex_identity.saturating_sub(1))
            .max(requested.next_vertex_identity.saturating_sub(1)),
    );
    merge_identity_floor(after, floor);
}

fn validate_unambiguous_feature_ids(
    configuration: &MechanicalConfiguration,
    label: &str,
) -> Result<(), String> {
    for (kind, ids) in [
        (
            MechanicalFeatureKind::Mount,
            configuration
                .mounts
                .iter()
                .map(|mount| mount.id.as_str())
                .collect::<Vec<_>>(),
        ),
        (
            MechanicalFeatureKind::ClosureMount,
            configuration
                .closure_mounts
                .iter()
                .flatten()
                .map(|mount| mount.id.as_str())
                .collect(),
        ),
        (
            MechanicalFeatureKind::Opening,
            configuration
                .openings
                .iter()
                .flatten()
                .map(|opening| opening.id.as_str())
                .collect(),
        ),
        (
            MechanicalFeatureKind::GasketSupport,
            configuration
                .gasket_layout
                .iter()
                .flat_map(|layout| &layout.supports)
                .map(|anchor| anchor.id.as_str())
                .collect(),
        ),
    ] {
        let mut seen = std::collections::BTreeSet::new();
        if let Some(duplicate) = ids.into_iter().find(|id| !seen.insert(*id)) {
            return Err(format!(
                "The {label} {kind:?} feature id {duplicate:?} is ambiguous."
            ));
        }
    }
    Ok(())
}

/// A replacement cannot recreate a member feature hidden by an Omitted exception.
/// Match the member's own `(kind, feature_id)`, because the pair relationship id can
/// legitimately differ from the identity on either physical member.
fn reject_omitted_reintroduction(
    document: &ProjectDoc,
    target: &MechanicalTarget,
    before: &MechanicalConfiguration,
    after: &MechanicalConfiguration,
) -> Result<(), String> {
    let MechanicalTarget::Instance { instance_id } = target else {
        return Ok(());
    };
    let Some(pair) = document.hardware.as_ref().and_then(|hardware| {
        hardware
            .pairs
            .iter()
            .find(|pair| pair.members.iter().any(|member| member == instance_id))
    }) else {
        return Ok(());
    };

    for feature in &pair.features {
        let Some(member) = feature
            .members
            .iter()
            .find(|member| member.instance_id == *instance_id)
        else {
            continue;
        };
        if member.state == MechanicalFeatureState::Omitted
            && find_feature_value(after, feature.kind, &member.feature_id).is_some()
        {
            return Err(format!(
                "Mechanical feature {:?} is omitted from physical instance {instance_id:?}; rejoin it through the feature lifecycle before replacing its configuration.",
                member.feature_id
            ));
        }
    }

    // A new item must not silently collide with a pair relationship or a reserved
    // member identity. Existing values are edited by their kind and member identity.
    for kind in [
        MechanicalFeatureKind::Mount,
        MechanicalFeatureKind::ClosureMount,
        MechanicalFeatureKind::Opening,
        MechanicalFeatureKind::GasketSupport,
    ] {
        for id in feature_ids(after, kind).difference(&feature_ids(before, kind)) {
            if pair.features.iter().any(|feature| {
                feature.id == *id
                    && feature
                        .members
                        .iter()
                        .find(|member| member.instance_id == *instance_id)
                        .is_none_or(|member| {
                            member.state != MechanicalFeatureState::Linked
                                || find_feature_value(after, feature.kind, &member.feature_id)
                                    .is_some()
                        })
            }) {
                return Err(format!(
                    "Mechanical feature id {id:?} is already used by a relationship in pair {:?}.",
                    pair.id
                ));
            }
            if let Some(member) = pair.features.iter().find_map(|feature| {
                (feature.kind == kind)
                    .then(|| {
                        feature.members.iter().find(|member| {
                            member.instance_id == *instance_id && member.feature_id == *id
                        })
                    })
                    .flatten()
            }) {
                return Err(format!(
                    "Mechanical feature id {id:?} is already reserved on physical instance {instance_id:?} as {:?}.",
                    member.state
                ));
            }
        }
    }
    Ok(())
}

fn replacement_affects_projection(
    before: &MechanicalConfiguration,
    after: &MechanicalConfiguration,
) -> bool {
    before.mount != after.mount
        || before.mounts != after.mounts
        || before.closure_mounts != after.closure_mounts
        || before.openings != after.openings
        || before.clearance != after.clearance
        || before.profiles != after.profiles
        || before.stabilizers != after.stabilizers
        || before.gasket_layout.as_ref().map(|layout| &layout.supports)
            != after.gasket_layout.as_ref().map(|layout| &layout.supports)
}

fn normalized_mounting_hole() -> Result<PartDefinition, String> {
    crate::generators::catalogue()?
        .into_iter()
        .find(|definition| {
            definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == "ceoloide/mounting_hole_npth")
        })
        .ok_or_else(|| "The normalized mounting-hole generator is unavailable.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        GasketPlacement, HardwareConfiguration, MechanicalConstructionOwnership, MechanicalFeature,
        MechanicalFeatureKind, MechanicalFeatureMember, MechanicalFeatureState,
        MechanicalFeatureValue, MechanicalGasketAnchor, MechanicalIdentityReservation,
        MechanicalMirrorPair, MechanicalMount, MirroredCompanionMode, MirroredHalves, Mount,
        MountKind, PhysicalBoardInstance, Pose2, Vec2,
    };
    use serde_json::json;

    fn config(board_id: &str) -> MechanicalConfiguration {
        serde_json::from_value(json!({
            "boardId": board_id,
            "method": "printed",
            "mount": "rigid",
            "plateThickness": 1.5,
            "plateFoamThickness": 0.5,
            "pcbThickness": 1.6,
            "bottomFoamThickness": 0.4,
            "batteryHeight": 4.0,
            "bottomThickness": 3.0,
            "plateToPcb": 3.5,
            "wallThickness": 2.0,
            "clearance": 0.3,
            "mounts": [],
            "closureMounts": [],
            "profiles": []
        }))
        .expect("valid mechanical fixture")
    }

    fn mount(id: &str, x: f64) -> Mount {
        Mount {
            id: id.into(),
            at: Vec2 { x, y: 2.0 },
            kind: MountKind::Hole,
            hole_diameter: 2.0,
            boss_diameter: None,
            height: None,
        }
    }

    fn instance(
        id: &str,
        board_id: &str,
        mechanical: Option<MechanicalConfiguration>,
    ) -> PhysicalBoardInstance {
        PhysicalBoardInstance {
            id: id.into(),
            name: id.into(),
            board_id: board_id.into(),
            half: id.into(),
            role: id.into(),
            flipped: false,
            controller_part_id: None,
            mechanical,
            construction_ownership: MechanicalConstructionOwnership::Pair,
            mechanical_identity_reservation: None,
        }
    }

    fn pair(feature: Option<MechanicalFeature>) -> MechanicalMirrorPair {
        let construction: crate::model::MechanicalConstruction = serde_json::from_value(json!({
            "method": "printed",
            "mount": "rigid",
            "integratedPlateFrame": false,
            "plateThickness": 1.5,
            "plateFoamThickness": 0.5,
            "bottomFoamThickness": 0.4,
            "bottomThickness": 3.0,
            "plateToPcb": 3.5,
            "wallThickness": 2.0,
            "clearance": 0.3
        }))
        .expect("valid construction fixture");
        MechanicalMirrorPair {
            id: "pair".into(),
            members: ["left".into(), "right".into()],
            mapping: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            construction,
            construction_source: Some("left".into()),
            halves: None,
            features: feature.into_iter().collect(),
        }
    }

    fn linked_mount() -> MechanicalFeature {
        MechanicalFeature {
            id: "relationship-mount".into(),
            kind: MechanicalFeatureKind::Mount,
            author: "left".into(),
            value: MechanicalFeatureValue::Mount {
                mount: mount("left-mount", 10.0),
            },
            members: vec![
                MechanicalFeatureMember {
                    instance_id: "left".into(),
                    feature_id: "left-mount".into(),
                    state: MechanicalFeatureState::Linked,
                },
                MechanicalFeatureMember {
                    instance_id: "right".into(),
                    feature_id: "left-mount".into(),
                    state: MechanicalFeatureState::Linked,
                },
            ],
        }
    }

    fn gasket_anchor(id: &str, anchor: f64) -> MechanicalGasketAnchor {
        MechanicalGasketAnchor {
            length: Some(12.0),
            width: Some(3.0),
            placement: Some(GasketPlacement::User),
            id: id.into(),
            region_id: "region".into(),
            outline_key: "outline".into(),
            anchor,
            unlinked: false,
        }
    }

    fn document(with_pair: bool) -> ProjectDoc {
        let mut document = ProjectDoc::empty("doc", "replacement fixture");
        document
            .boards
            .extend(["left-board", "right-board"].map(|id| Board {
                id: id.into(),
                name: id.into(),
                outline_ids: Vec::new(),
                part_ids: Vec::new(),
                net_ids: Vec::new(),
                thickness: 1.6,
                traces: Vec::new(),
                vias: Vec::new(),
            }));
        let left = config("left-board");
        let mut right = config("right-board");
        let feature = with_pair.then(linked_mount);
        if let Some(feature) = &feature {
            document.hardware = Some(HardwareConfiguration {
                instances: vec![
                    instance("left", "left-board", Some(left)),
                    instance("right", "right-board", Some(right)),
                ],
                pairs: vec![pair(Some(feature.clone()))],
                ..HardwareConfiguration::default()
            });
            return document;
        }
        // Give the unpaired instance an ordinary stored stack as well.
        right.mount = MechanicalMount::Rigid;
        document.hardware = Some(HardwareConfiguration {
            instances: vec![
                instance("left", "left-board", Some(left)),
                instance("right", "right-board", Some(right)),
            ],
            ..HardwareConfiguration::default()
        });
        document
    }

    fn target(id: &str) -> MechanicalTarget {
        MechanicalTarget::Instance {
            instance_id: id.into(),
        }
    }

    fn effective(document: &ProjectDoc, id: &str) -> MechanicalConfiguration {
        effective_inputs(document, &[], &target(id))
            .expect("effective instance configuration")
            .configuration
    }

    fn stored<'a>(document: &'a ProjectDoc, id: &str) -> Option<&'a MechanicalConfiguration> {
        document
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == id)
            .unwrap()
            .mechanical
            .as_ref()
    }

    fn replace(
        document: &mut ProjectDoc,
        id: &str,
        requested: Option<MechanicalConfiguration>,
    ) -> Result<Vec<String>, String> {
        apply_instance_configuration(document, &[], &target(id), requested)
    }

    #[test]
    fn pair_replacement_updates_shared_construction_but_keeps_local_settings_local() {
        let mut document = document(true);
        let mut replacement = effective(&document, "right");
        replacement.plate_thickness = 2.2;
        replacement.battery_height = 8.0;

        replace(&mut document, "right", Some(replacement)).expect("replacement succeeds");

        let pair = &document.hardware.as_ref().unwrap().pairs[0];
        assert_eq!(pair.construction.plate_thickness, 2.2);
        assert_eq!(stored(&document, "left").unwrap().plate_thickness, 1.5);
        assert_eq!(stored(&document, "right").unwrap().plate_thickness, 2.2);
        assert_eq!(stored(&document, "right").unwrap().battery_height, 8.0);
        assert_eq!(stored(&document, "left").unwrap().battery_height, 4.0);
    }

    #[test]
    fn linked_mount_can_be_added_edited_and_deleted_from_either_member() {
        for id in ["left", "right"] {
            let mut document = document(true);
            document.hardware.as_mut().unwrap().pairs[0].features[0].members[1].feature_id =
                "right-mount".into();
            let selected_id = if id == "left" {
                "left-mount"
            } else {
                "right-mount"
            };
            let (other, other_id) = if id == "left" {
                ("right", "right-mount")
            } else {
                ("left", "left-mount")
            };
            let mut replacement = effective(&document, id);
            replacement.mounts.push(mount("new-mount", 15.0));
            replacement
                .mounts
                .iter_mut()
                .find(|mount| mount.id == selected_id)
                .unwrap()
                .at
                .x = 18.0;
            replace(&mut document, id, Some(replacement)).expect("linked edit succeeds");
            assert_eq!(
                document.hardware.as_ref().unwrap().pairs[0].features[0].author,
                id
            );
            let counterpart = effective(&document, other);
            assert_eq!(
                counterpart
                    .mounts
                    .iter()
                    .find(|mount| mount.id == other_id)
                    .unwrap()
                    .at,
                Vec2 { x: -18.0, y: 2.0 }
            );
            assert_eq!(
                counterpart
                    .mounts
                    .iter()
                    .find(|mount| mount.id == "new-mount")
                    .unwrap()
                    .at,
                Vec2 { x: -15.0, y: 2.0 }
            );
            assert_eq!(
                document.hardware.as_ref().unwrap().pairs[0].features.len(),
                2
            );

            let mut replacement = effective(&document, id);
            replacement.mounts.clear();
            replace(&mut document, id, Some(replacement)).expect("linked deletion succeeds");
            assert!(
                document.hardware.as_ref().unwrap().pairs[0]
                    .features
                    .is_empty()
            );
            assert!(effective(&document, id).mounts.is_empty());
            assert!(effective(&document, other).mounts.is_empty());
        }
    }

    #[test]
    fn local_mount_remains_independent_from_pair_features() {
        let mut document = document(true);
        document.hardware.as_mut().unwrap().pairs[0].features[0].members[1].state =
            MechanicalFeatureState::Local;
        document.hardware.as_mut().unwrap().instances[1]
            .mechanical
            .as_mut()
            .unwrap()
            .mounts = vec![mount("left-mount", -25.0)];
        let before_feature = document.hardware.as_ref().unwrap().pairs[0].features[0].clone();
        let mut replacement = effective(&document, "right");
        replacement.mounts[0].at.x = -30.0;

        replace(&mut document, "right", Some(replacement)).expect("local mount edit succeeds");

        let hardware = document.hardware.as_ref().unwrap();
        assert_eq!(hardware.pairs[0].features.len(), 1);
        assert_eq!(hardware.pairs[0].features[0], before_feature);
        assert_eq!(effective(&document, "right").mounts[0].at.x, -30.0);
        assert_eq!(effective(&document, "left").mounts[0].at.x, 10.0);
        assert_eq!(stored(&document, "right").unwrap().mounts[0].at.x, -30.0);
    }

    #[test]
    fn linked_counterpart_uses_the_member_feature_identity() {
        let mut document = document(true);
        document.hardware.as_mut().unwrap().pairs[0].features[0].members[1].feature_id =
            "right-mount".into();

        let configuration = effective(&document, "right");

        assert!(
            configuration
                .mounts
                .iter()
                .any(|mount| mount.id == "right-mount")
        );
    }

    #[test]
    fn omitted_member_identity_is_refused_even_when_relationship_id_differs() {
        let mut document = document(true);
        let pair = &mut document.hardware.as_mut().unwrap().pairs[0];
        pair.features[0].members[1].state = MechanicalFeatureState::Omitted;
        pair.features[0].members[1].feature_id = "right-mount".into();
        let mut replacement = effective(&document, "right");
        replacement.mounts.push(mount("right-mount", 10.0));
        let before = document.clone();

        let error = replace(&mut document, "right", Some(replacement)).unwrap_err();

        assert!(error.contains("right-mount"));
        assert!(error.contains("omitted"), "{error}");
        assert_eq!(document, before);
    }

    #[test]
    fn replacement_preserves_identity_reservations_from_accepted_geometry() {
        let mut document = document(false);
        let opening: MechanicalOpening = serde_json::from_value(json!({
            "id": "opening-3",
            "vertices": [
                {"id":"vertex-5", "point":{"x":0.0,"y":0.0}},
                {"id":"vertex-6", "point":{"x":1.0,"y":0.0}},
                {"id":"vertex-7", "point":{"x":0.0,"y":1.0}}
            ], "z":0.0, "height":2.0, "nextVertexIdentity":1
        }))
        .unwrap();
        document.hardware.as_mut().unwrap().instances[0]
            .mechanical
            .as_mut()
            .unwrap()
            .openings = Some(vec![opening.clone()]);
        let mut replacement = effective(&document, "left");
        replacement.openings = Some(vec![opening]);
        replacement.next_opening_identity = 1;

        replace(&mut document, "left", Some(replacement)).expect("identity reservation succeeds");

        let accepted = stored(&document, "left").unwrap();
        assert_eq!(accepted.next_opening_identity, 4);
        assert_eq!(
            accepted.openings.as_ref().unwrap()[0].next_vertex_identity,
            8
        );
    }

    #[test]
    fn disabling_one_member_unlinks_pair_and_materializes_the_survivor() {
        let mut document = document(true);

        replace(&mut document, "left", None).expect("disable succeeds");

        let hardware = document.hardware.as_ref().unwrap();
        assert!(hardware.pairs.is_empty());
        assert!(stored(&document, "left").is_none());
        assert!(stored(&document, "right").is_some());
        assert_eq!(
            stored(&document, "right").unwrap().mounts[0].id,
            "left-mount"
        );
    }

    #[test]
    fn disabling_legacy_pair_member_without_a_slot_still_dissolves_pair_ownership() {
        let mut document = document(false);
        let hardware = document.hardware.as_mut().unwrap();
        hardware.instances[0].mechanical = None;
        hardware.instances[0].construction_ownership = MechanicalConstructionOwnership::Legacy;
        hardware.shared_construction = Some(config("shared-board"));
        hardware.pairs = vec![pair(None)];

        replace(&mut document, "left", None).expect("pair-owned legacy stack disables");

        let hardware = document.hardware.as_ref().unwrap();
        assert!(hardware.pairs.is_empty());
        assert!(stored(&document, "left").is_none());
        assert!(stored(&document, "right").is_some());
    }

    #[test]
    fn reuse_board_pair_replacement_is_processed_once_without_board_slot_duplication() {
        let mut document = document(true);
        let hardware = document.hardware.as_mut().unwrap();
        hardware.instances[1].board_id = "left-board".into();
        hardware.instances[1].mechanical.as_mut().unwrap().board_id = "left-board".into();
        hardware.pairs[0].halves = Some(MirroredHalves {
            companion_mode: MirroredCompanionMode::LiteralMirror,
            source_board_id: "left-board".into(),
            mirror_board_id: "left-board".into(),
            correspondences: Vec::new(),
            exceptions: Vec::new(),
        });
        let mut replacement = effective(&document, "right");
        replacement.battery_height = 7.0;

        replace(&mut document, "right", Some(replacement))
            .expect("reuse-board replacement succeeds");

        let hardware = document.hardware.as_ref().unwrap();
        assert_eq!(hardware.pairs.len(), 1);
        assert_eq!(hardware.pairs[0].construction.plate_thickness, 1.5);
        assert_eq!(stored(&document, "right").unwrap().battery_height, 7.0);
        assert_eq!(stored(&document, "left").unwrap().battery_height, 4.0);
        assert!(document.mechanical.is_none());
        assert!(hardware.shared_construction.is_none());
    }

    #[test]
    fn unpaired_replacement_and_absent_disable_keep_ordinary_instance_semantics() {
        let mut document = document(false);
        let mut replacement = effective(&document, "left");
        replacement.battery_height = 6.0;
        replace(&mut document, "left", Some(replacement)).expect("unpaired replacement succeeds");
        assert_eq!(stored(&document, "left").unwrap().battery_height, 6.0);

        document.hardware.as_mut().unwrap().instances[0].mechanical = None;
        let before = document.clone();
        assert!(replace(&mut document, "left", None).unwrap().is_empty());
        assert_eq!(document, before);
    }

    #[test]
    fn supplied_configuration_enables_a_disabled_unpaired_instance() {
        let mut document = document(false);
        let hardware = document.hardware.as_mut().unwrap();
        let right = hardware
            .instances
            .iter_mut()
            .find(|instance| instance.id == "right")
            .unwrap();
        right.mechanical = None;
        right.construction_ownership = MechanicalConstructionOwnership::Local;
        right.mechanical_identity_reservation = Some(MechanicalIdentityReservation {
            next_opening_identity: 9,
            next_vertex_identity: 12,
        });
        let mut requested = config("right-board");
        requested.battery_height = 9.25;
        requested.plate_thickness = 2.75;
        requested.next_opening_identity = 1;

        replace(&mut document, "right", Some(requested)).expect("disabled instance enables");

        let stored = stored(&document, "right").expect("configuration is persisted");
        assert_eq!(stored.battery_height, 9.25);
        assert_eq!(stored.plate_thickness, 2.75);
        assert_eq!(stored.next_opening_identity, 9);
    }

    #[test]
    fn legacy_instance_without_a_slot_replaces_from_its_shared_source() {
        let mut document = document(false);
        let hardware = document.hardware.as_mut().unwrap();
        hardware.instances[1].mechanical = None;
        hardware.instances[1].construction_ownership = MechanicalConstructionOwnership::Legacy;
        hardware.shared_construction = Some(config("right-board"));
        let mut requested = effective(&document, "right");
        requested.wall_thickness = 2.75;

        replace(&mut document, "right", Some(requested)).expect("legacy replacement succeeds");

        assert_eq!(
            document
                .hardware
                .as_ref()
                .unwrap()
                .shared_construction
                .as_ref()
                .unwrap()
                .wall_thickness,
            2.75
        );
        assert_eq!(effective(&document, "right").wall_thickness, 2.75);
    }

    #[test]
    fn legacy_instance_without_explicit_shared_construction_persists_to_inherited_source() {
        let mut document = document(false);
        let hardware = document.hardware.as_mut().unwrap();
        hardware.instances[1].mechanical = None;
        hardware.instances[1].construction_ownership = MechanicalConstructionOwnership::Legacy;
        assert!(hardware.shared_construction.is_none());
        let sibling_battery = stored(&document, "left").unwrap().battery_height;
        let mut requested = effective(&document, "right");
        requested.wall_thickness = 2.75;
        requested.battery_height = 9.25;

        let changed = replace(&mut document, "right", Some(requested))
            .expect("legacy replacement uses its inherited source");

        assert_eq!(effective(&document, "right").wall_thickness, 2.75);
        assert_eq!(effective(&document, "right").battery_height, 9.25);
        assert_eq!(
            stored(&document, "left").unwrap().battery_height,
            sibling_battery
        );
        assert!(changed.iter().any(|id| id == "right"));
        assert!(changed.iter().any(|id| id == "left"), "{changed:?}");
    }

    #[test]
    fn invalid_local_gasket_support_anchors_refuse_atomically() {
        for invalid_anchor in [f64::NAN, -0.01, 1.01] {
            let mut document = document(true);
            let support = MechanicalFeature {
                id: "relationship-support".into(),
                kind: MechanicalFeatureKind::GasketSupport,
                author: "left".into(),
                value: MechanicalFeatureValue::GasketSupport {
                    anchor: gasket_anchor("left-support", 0.5),
                },
                members: vec![
                    MechanicalFeatureMember {
                        instance_id: "left".into(),
                        feature_id: "left-support".into(),
                        state: MechanicalFeatureState::Linked,
                    },
                    MechanicalFeatureMember {
                        instance_id: "right".into(),
                        feature_id: "right-support".into(),
                        state: MechanicalFeatureState::Local,
                    },
                ],
            };
            document.hardware.as_mut().unwrap().pairs[0]
                .features
                .push(support.clone());
            let right = &mut document.hardware.as_mut().unwrap().instances[1];
            let configuration = right.mechanical.as_mut().unwrap();
            configuration.mount = MechanicalMount::Gasket;
            configuration.gasket_layout = Some(
                serde_json::from_value(json!({
                    "length": 10.0,
                    "width": 2.0,
                    "thickness": 1.0,
                    "compression": 0.2,
                    "supports": [{
                        "id": "right-support",
                        "regionId": "region",
                        "outlineKey": "outline",
                        "anchor": 0.5,
                        "length": 12.0,
                        "width": 3.0,
                        "placement": "user"
                    }]
                }))
                .expect("valid local gasket support fixture"),
            );
            let before = document.clone();
            let mut requested = effective(&document, "right");
            requested.gasket_layout.as_mut().unwrap().supports[0].anchor = invalid_anchor;

            let error = replace(&mut document, "right", Some(requested)).unwrap_err();

            assert!(
                error.contains("gasket support placement is invalid"),
                "{error}"
            );
            assert_eq!(document, before);
            assert_eq!(
                document.hardware.as_ref().unwrap().pairs[0].features[1],
                support
            );
        }
    }

    #[test]
    fn mismatched_configuration_does_not_partially_enable_a_disabled_instance() {
        let mut document = document(false);
        let right = &mut document.hardware.as_mut().unwrap().instances[1];
        right.mechanical = None;
        right.construction_ownership = MechanicalConstructionOwnership::Local;
        let before = document.clone();
        let requested = config("wrong-board");

        let error = replace(&mut document, "right", Some(requested)).unwrap_err();

        assert!(
            error.contains("does not match physical instance"),
            "{error}"
        );
        assert_eq!(document, before);
    }

    #[test]
    fn invalid_mount_does_not_partially_enable_a_disabled_instance() {
        let mut document = document(false);
        let right = &mut document.hardware.as_mut().unwrap().instances[1];
        right.mechanical = None;
        right.construction_ownership = MechanicalConstructionOwnership::Local;
        let before = document.clone();
        let mut requested = config("right-board");
        requested.mounts.push(mount("invalid-mount", f64::NAN));

        let error = replace(&mut document, "right", Some(requested)).unwrap_err();

        assert!(error.contains("Mount positions must be finite"), "{error}");
        assert_eq!(document, before);
    }

    #[test]
    fn rejected_replacement_is_atomic_and_board_mismatch_is_reported() {
        let mut document = document(false);
        let mut replacement = effective(&document, "left");
        replacement.board_id = "wrong-board".into();
        let before = document.clone();

        let error = replace(&mut document, "left", Some(replacement)).unwrap_err();

        assert!(error.contains("does not match physical instance"));
        assert_eq!(document, before);
    }

    #[test]
    fn late_validation_refusal_preserves_pair_construction_and_features() {
        let mut document = document(true);
        let before = document.clone();
        let mut replacement = effective(&document, "right");
        replacement.wall_thickness = 3.25;
        let mut invalid = mount("invalid-mount", 12.0);
        // A Hole's unused boss height passes the typed mount geometry validator,
        // but final pair validation still requires the stored value to be finite.
        invalid.height = Some(f64::NAN);
        replacement.mounts.push(invalid);

        let error = replace(&mut document, "right", Some(replacement)).unwrap_err();

        assert!(error.contains("non-finite mount height"), "{error}");
        assert_eq!(
            document, before,
            "the disposable draft owns the entire refusal"
        );
    }

    #[test]
    fn ambiguous_pair_ownership_refuses_instance_clear_atomically() {
        let mut document = document(true);
        let hardware = document.hardware.as_mut().unwrap();
        let mut duplicate = hardware.pairs[0].clone();
        duplicate.id = "other-pair".into();
        hardware.pairs.push(duplicate);
        let before = document.clone();

        let error = replace(&mut document, "left", None).unwrap_err();

        assert!(error.contains("belongs to mechanical pairs"), "{error}");
        assert_eq!(document, before);
    }
}
