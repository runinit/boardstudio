use super::*;
use crate::mechanical::change::{
    MechanicalChange, MechanicalDimension, MechanicalFeatureCorrespondence,
};
use crate::mechanical::features;
use crate::model::{
    HardwareConfiguration, HardwareTopology, HardwareTransport, MechanicalConfiguration,
    MechanicalConstructionOwnership, MechanicalFeatureKind, MechanicalFeatureState,
    MechanicalFeatureValue, MechanicalMirrorPair, MountKind, PhysicalBoardInstance, Pose2, Vec2,
};
use crate::test_support::*;
use serde_json::json;

/// The same configuration with absent closure mounts, for initialization tests.
fn configuration_without_closures() -> MechanicalConfiguration {
    let mut configuration = edit_configuration();
    configuration.closure_mounts = None;
    configuration
}

/// The normalized bundled mounting-hole catalogue entry projection expects.
fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board("board", vec![], 1.6));
    document.mechanical = Some(edit_configuration());
    document
}

/// A board whose outline yields at least one suggested closure mount.
fn document_with_outline() -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.outline.push(OutlineFeature::Rect {
        anchor_part_id: None,
        id: "outline".into(),
        center: Vec2 { x: 0.0, y: 0.0 },
        size: Vec2 { x: 80.0, y: 60.0 },
        radius: 0.0,
        rotation: None,
        operation: Operation::Add,
    });
    document.boards.push(board("board", vec![], 1.6));
    document.boards[0].outline_ids = vec!["outline".into()];
    document.mechanical = Some(configuration_without_closures());
    document
}

fn board_target() -> MechanicalTarget {
    MechanicalTarget::Board {
        board_id: "board".into(),
    }
}

fn instance_target() -> MechanicalTarget {
    MechanicalTarget::Instance {
        instance_id: "left".into(),
    }
}

fn apply_change(doc: &mut ProjectDoc, change: MechanicalChange) -> Result<Vec<String>, String> {
    apply(
        doc,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change,
            mounting_hole: Some(Box::new(mounting_hole_template())),
        },
    )
}

fn stored(doc: &ProjectDoc) -> &MechanicalConfiguration {
    doc.mechanical.as_ref().expect("stored configuration")
}

#[test]
fn the_initial_switch_family_follows_generator_inference_and_part_overrides() {
    let choc = "ceoloide/switch_choc_v1_v2";
    type Generator<'a> = Option<(&'a str, serde_json::Value)>;
    let v1_params = json!({"choc_v1_support":true,"choc_v2_support":false});
    let family_document = |parts: &[(&str, &str, Option<serde_json::Value>)],
                           definitions: &[(&str, &str, Generator)]| {
        let mut doc = ProjectDoc::empty("family", "family");
        doc.boards.push(board(
            "board",
            parts.iter().map(|(id, _, _)| id.to_string()).collect(),
            1.6,
        ));
        for (id, definition_id, parameters) in parts {
            let mut part = json!({
                "id":id, "definitionId":definition_id, "reference":id,
                "pose":{"at":{"x":0,"y":0},"rotation":0}, "side":"front"
            });
            if let Some(parameters) = parameters {
                part["generatorParameters"] = parameters.clone();
            }
            doc.parts.push(serde_json::from_value(part).unwrap());
        }
        for (id, kind, generator) in definitions {
            let mut definition = json!({
                "id":id, "name":id, "kind":kind, "courtyard":[], "pads":[]
            });
            if let Some((source, parameters)) = generator {
                definition["generator"] =
                    json!({"source":source,"version":"1","parameters":parameters});
            }
            doc.definitions
                .push(serde_json::from_value(definition).unwrap());
        }
        doc
    };
    use MechanicalSwitchFamily::{ChocV1, ChocV2, Mx};
    for (doc, expected) in [
        (
            family_document(
                &[("a", "mx", None)],
                &[("mx", "switch", Some(("ceoloide/switch_mx", json!({}))))],
            ),
            Some(Mx),
        ),
        // A family-bearing generator admits a part whose kind is not a switch.
        (
            family_document(
                &[("a", "mx", None)],
                &[("mx", "controller", Some(("Ceoloide/Switch_MX", json!({}))))],
            ),
            Some(Mx),
        ),
        (
            family_document(
                &[(
                    "a",
                    "c",
                    Some(json!({"choc_v1_support":false,"choc_v2_support":true})),
                )],
                &[("c", "switch", Some((choc, v1_params.clone())))],
            ),
            Some(ChocV2),
        ),
        (
            family_document(
                &[("a", "c", None)],
                &[("c", "switch", Some((choc, json!({}))))],
            ),
            None,
        ),
        (
            family_document(
                &[("a", "mx", None), ("b", "c", None)],
                &[
                    ("mx", "switch", Some(("ceoloide/switch_mx", json!({})))),
                    ("c", "switch", Some((choc, v1_params.clone()))),
                ],
            ),
            None,
        ),
        (
            family_document(
                &[("a", "c", None), ("b", "c", None)],
                &[("c", "switch", Some((choc, v1_params)))],
            ),
            Some(ChocV1),
        ),
        (
            family_document(&[("a", "s", None)], &[("s", "switch", None)]),
            None,
        ),
        (
            family_document(&[("a", "k", None)], &[("k", "controller", None)]),
            None,
        ),
    ] {
        assert_eq!(initial_switch_family(&doc, "board"), expected);
    }
}

#[test]
fn reflected_feature_installation_preserves_each_members_identity() {
    let opening = MechanicalOpening {
        id: "left-feature".into(),
        vertices: vec![MechanicalOpeningVertex {
            id: "vertex-4".into(),
            point: Vec2 { x: 2.0, y: 3.0 },
        }],
        z: 1.0,
        height: 5.0,
        next_vertex_identity: 5,
    };
    let mount = MechanicalFeatureValue::Mount {
        mount: mount("left-feature", 8.0, 9.0),
    };
    let values = [
        (MechanicalFeatureKind::Mount, mount.clone()),
        (MechanicalFeatureKind::ClosureMount, mount),
        (
            MechanicalFeatureKind::Opening,
            MechanicalFeatureValue::Opening { opening },
        ),
        (
            MechanicalFeatureKind::GasketSupport,
            MechanicalFeatureValue::GasketSupport {
                anchor: {
                    let mut anchor = anchor_at("left-feature", 0.25);
                    anchor.region_id = anchor.id.clone();
                    anchor.outline_key = anchor.id.clone();
                    anchor
                },
            },
        ),
    ];
    for (kind, value) in values {
        let mut configuration = edit_configuration();
        install_feature(&mut configuration, kind, "right-feature", &value).unwrap();
        let installed = find_feature_value(&configuration, kind, "right-feature")
            .unwrap_or_else(|| panic!("the {kind:?} counterpart must use its member identity"));
        match (&value, installed) {
            (
                MechanicalFeatureValue::Mount { mount: authored },
                MechanicalFeatureValue::Mount { mount: installed },
            ) => {
                assert_eq!(installed.id, "right-feature");
                assert_eq!(installed.at, authored.at);
                assert_eq!(installed.hole_diameter, authored.hole_diameter);
            }
            (
                MechanicalFeatureValue::Opening { opening: authored },
                MechanicalFeatureValue::Opening { opening: installed },
            ) => {
                assert_eq!(installed.id, "right-feature");
                assert_eq!(installed.vertices, authored.vertices);
                assert_eq!(installed.z, authored.z);
                assert_eq!(installed.height, authored.height);
                assert_eq!(
                    installed.next_vertex_identity,
                    authored.next_vertex_identity
                );
            }
            (
                MechanicalFeatureValue::GasketSupport { anchor: authored },
                MechanicalFeatureValue::GasketSupport { anchor: installed },
            ) => {
                assert_eq!(installed.id, "right-feature");
                assert_eq!(installed.region_id, authored.region_id);
                assert_eq!(installed.outline_key, authored.outline_key);
                assert_eq!(installed.anchor, authored.anchor);
            }
            _ => panic!("installation must preserve the feature kind"),
        }
        assert!(find_feature_value(&configuration, kind, "left-feature").is_none());
        let once = configuration.clone();
        install_feature(&mut configuration, kind, "right-feature", &value).unwrap();
        assert_eq!(
            configuration, once,
            "reinstallation replaces the same member"
        );
    }
}

#[test]
fn a_removed_opening_or_vertex_retires_a_later_queued_edit_without_redirecting_it() {
    // An earlier queued edit addressed to a surviving vertex must still hit
    // that vertex after a different opening was removed before it.
    let mut doc = document();
    apply_change(&mut doc, MechanicalChange::AddOpening).unwrap();
    apply_change(&mut doc, MechanicalChange::AddOpening).unwrap();
    assert_eq!(
        stored(&doc)
            .openings
            .as_ref()
            .unwrap()
            .iter()
            .map(|opening| opening.id.as_str())
            .collect::<Vec<_>>(),
        vec!["opening-1", "opening-2"]
    );
    apply_change(
        &mut doc,
        MechanicalChange::SetOpeningDimension {
            opening_id: "opening-2".into(),
            vertex_id: Some("vertex-1".into()),
            field: MechanicalDimension::OpeningPointX,
            value: 4.0,
        },
    )
    .unwrap();
    apply_change(
        &mut doc,
        MechanicalChange::RemoveOpening {
            opening_id: "opening-1".into(),
        },
    )
    .unwrap();
    // The surviving opening keeps its durable id even though it is now index 0.
    assert_eq!(stored(&doc).openings.as_ref().unwrap().len(), 1);
    apply_change(
        &mut doc,
        MechanicalChange::SetOpeningDimension {
            opening_id: "opening-2".into(),
            vertex_id: Some("vertex-1".into()),
            field: MechanicalDimension::OpeningPointX,
            value: 9.0,
        },
    )
    .unwrap();
    let opening = &stored(&doc).openings.as_ref().unwrap()[0];
    assert_eq!(opening.id, "opening-2");
    assert_eq!(opening.vertices[0].id, "vertex-1");
    assert_eq!(opening.vertices[0].point.x, 9.0);

    // An edit whose addressed vertex was removed is a Core error and mutates nothing.
    apply_change(
        &mut doc,
        MechanicalChange::RemoveOpeningVertex {
            opening_id: "opening-2".into(),
            vertex_id: "vertex-1".into(),
        },
    )
    .unwrap();
    let before = stored(&doc).clone();
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::SetOpeningDimension {
                opening_id: "opening-2".into(),
                vertex_id: Some("vertex-1".into()),
                field: MechanicalDimension::OpeningPointX,
                value: 5.0,
            },
        )
        .is_err(),
        "a removed vertex id is no longer a valid target"
    );
    assert_eq!(stored(&doc), &before);
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::SetOpeningDimension {
                opening_id: "opening-1".into(),
                vertex_id: Some("vertex-1".into()),
                field: MechanicalDimension::OpeningPointX,
                value: 5.0,
            },
        )
        .is_err(),
        "a removed opening id is no longer a valid target"
    );
    assert_eq!(stored(&doc), &before);
}

/// Case 1: absent closure mounts initialize from the resolved assembly, and an
/// explicit `Some(vec![])` stays deliberate.
#[test]
fn absent_closure_mounts_initialize_from_the_resolved_assembly() {
    let mut doc = document_with_outline();
    assert!(stored(&doc).closure_mounts.is_none());
    let changed = apply_change(&mut doc, MechanicalChange::InitializeClosures).unwrap();
    let mounts = stored(&doc)
        .closure_mounts
        .clone()
        .expect("initialized mounts");
    assert!(!mounts.is_empty(), "the outline suggests closure mounts");
    for mount in &mounts {
        assert!(mount.id.starts_with("auto-closure/"));
        assert_eq!(mount.kind, MountKind::Boss);
        assert_eq!(mount.hole_diameter, 2.2);
        assert_eq!(
            mount.height,
            Some(
                stored(&doc).plate_to_pcb
                    + stored(&doc).pcb_thickness
                    + stored(&doc)
                        .bottom_foam_thickness
                        .max(stored(&doc).battery_height)
            )
        );
    }
    assert!(changed.contains(&"board".to_string()));

    // Explicit empty is deliberate and never re-initialized.
    let mut deliberate = document_with_outline();
    deliberate.mechanical.as_mut().unwrap().closure_mounts = Some(vec![]);
    assert!(
        apply_change(&mut deliberate, MechanicalChange::InitializeClosures).is_err(),
        "an explicit empty collection is not eligible for initialization"
    );
    assert_eq!(stored(&deliberate).closure_mounts, Some(vec![]));
}

/// Case 2 and 4: generated clearance parts, definitions and memberships appear
/// with the configuration owner and nothing else.
#[test]
fn initialization_reports_only_generated_ids_and_the_owner() {
    let mut doc = document_with_outline();
    let changed = apply_change(&mut doc, MechanicalChange::InitializeClosures).unwrap();
    assert_eq!(changed[0], "board", "the configuration owner leads");
    assert!(!doc.parts.is_empty(), "generated clearance parts exist");
    for part in &doc.parts {
        assert!(
            changed.contains(&part.id),
            "generated part {} is reported",
            part.id
        );
    }
    for definition in &doc.definitions {
        assert!(changed.contains(&definition.id));
    }
    assert!(changed.contains(&"board".to_string()));
    for id in &changed {
        assert!(
            id == "board"
                || id.starts_with("case-closure/")
                || id.starts_with("assembly-closure/definition/"),
            "unexpected changed id {id}"
        );
    }
}

/// Case 5: a required projection without a template rolls back the configuration.
///
/// The stored closure mount makes the projection genuinely create a generated
/// hole, so the normalized entry is required; a change whose projected hole set is
/// empty would not need one.
#[test]
fn a_missing_required_template_rolls_back_the_configuration() {
    let mut doc = document();
    doc.mechanical.as_mut().unwrap().closure_mounts = Some(vec![mount("closure-1", 4.0, 0.0)]);
    let before = doc.clone();
    let error = apply(
        &mut doc,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::AddOpening,
            mounting_hole: None,
        },
    )
    .unwrap_err();
    assert!(error.contains("mounting-hole"), "{error}");
    assert_eq!(doc, before, "the configuration change rolled back");
}

fn mirror_instance(id: &str, mount_x: f64) -> PhysicalBoardInstance {
    let mut mechanical = edit_configuration();
    mechanical.mounts = vec![mount("mount-1", mount_x, 3.0)];
    PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: "board".into(),
        half: id.into(),
        role: "controller".into(),
        flipped: false,
        controller_part_id: None,
        mechanical: Some(mechanical),
        construction_ownership: MechanicalConstructionOwnership::Local,
        mechanical_identity_reservation: None,
    }
}

/// Two physical halves on one board, each with its own local mount geometry.
fn mirror_document() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("doc", "Fixture");
    doc.boards.push(board("board", vec![], 1.6));
    doc.mechanical = Some(edit_configuration());
    doc.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![mirror_instance("left", 2.0), mirror_instance("right", 7.0)],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    doc
}

fn correspondence() -> MechanicalFeatureCorrespondence {
    MechanicalFeatureCorrespondence {
        kind: MechanicalFeatureKind::Mount,
        first_feature_id: "mount-1".into(),
        second_feature_id: "mount-1".into(),
        source_instance_id: None,
    }
}

fn link_edit(target: MechanicalTarget, pair_id: &str, source: &str) -> MechanicalEdit {
    MechanicalEdit {
        target,
        change: MechanicalChange::LinkMirrorPair {
            pair_id: pair_id.into(),
            members: ["left".into(), "right".into()],
            mapping: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            construction_source: source.into(),
            correspondences: vec![correspondence()],
        },
        mounting_hole: Some(Box::new(mounting_hole_template())),
    }
}

fn lifecycle_edit(change: MechanicalChange) -> MechanicalEdit {
    MechanicalEdit {
        target: instance_target(),
        change,
        mounting_hole: Some(Box::new(mounting_hole_template())),
    }
}

fn pairs(doc: &ProjectDoc) -> &[MechanicalMirrorPair] {
    &doc.hardware.as_ref().unwrap().pairs
}

fn stored_instance<'a>(doc: &'a ProjectDoc, id: &str) -> &'a MechanicalConfiguration {
    doc.hardware
        .as_ref()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.id == id)
        .unwrap()
        .mechanical
        .as_ref()
        .unwrap()
}

fn ownership(doc: &ProjectDoc, id: &str) -> MechanicalConstructionOwnership {
    doc.hardware
        .as_ref()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.id == id)
        .unwrap()
        .construction_ownership
}

fn mount_at(configuration: &MechanicalConfiguration, id: &str) -> Vec2 {
    configuration
        .mounts
        .iter()
        .find(|mount| mount.id == id)
        .unwrap()
        .at
}

/// The visible geometry a physical half shows for a mount, resolved through the
/// effective read path (pair-derived when the feature is linked).
fn visible_mount(doc: &ProjectDoc, instance_id: &str, id: &str) -> Vec2 {
    mount_at(
        &effective_inputs(
            doc,
            &[],
            &MechanicalTarget::Instance {
                instance_id: instance_id.into(),
            },
        )
        .unwrap()
        .configuration,
        id,
    )
}

/// Whether a physical half stores its own copy of a feature.
///
/// A `Linked` feature is owned by the pair, so neither half may store a second
/// writable copy that could drift from the authored value.
fn stores_mount(doc: &ProjectDoc, instance_id: &str, id: &str) -> bool {
    stored_instance(doc, instance_id)
        .mounts
        .iter()
        .any(|mount| mount.id == id)
}

fn linked_document() -> ProjectDoc {
    let mut doc = mirror_document();
    apply(
        &mut doc,
        &[],
        &link_edit(board_target(), "pair-left-right", "left"),
    )
    .unwrap();
    doc
}

#[test]
fn linking_a_pair_shares_construction_and_sets_both_ownerships() {
    let mut doc = mirror_document();
    let changed = apply(
        &mut doc,
        &[],
        &link_edit(board_target(), "pair-left-right", "left"),
    )
    .unwrap();
    assert_eq!(changed, vec!["pair-left-right", "left", "right"]);
    let pair = &pairs(&doc)[0];
    assert_eq!(pair.members, ["left".to_string(), "right".to_string()]);
    // The source's shared construction excludes its board-local mounts.
    assert_eq!(
        pair.construction,
        features::shared_construction(&edit_configuration())
    );
    assert_eq!(pair.features.len(), 1);
    assert_eq!(pair.features[0].author, "left");
    assert_eq!(
        features::member_state(pair, "left", "mount-1"),
        Some(MechanicalFeatureState::Linked)
    );
    assert_eq!(
        features::member_state(pair, "right", "mount-1"),
        Some(MechanicalFeatureState::Linked)
    );
    assert_eq!(
        ownership(&doc, "left"),
        MechanicalConstructionOwnership::Pair
    );
    assert_eq!(
        ownership(&doc, "right"),
        MechanicalConstructionOwnership::Pair
    );
    // The pair owns the single authored value. Both halves read it - the author
    // its stored value and the other half the reflected counterpart - and neither
    // keeps a second writable copy.
    assert_point(
        visible_mount(&doc, "left", "mount-1"),
        Vec2 { x: 2.0, y: 3.0 },
    );
    assert_point(
        visible_mount(&doc, "right", "mount-1"),
        Vec2 { x: -2.0, y: 3.0 },
    );
    assert!(!stores_mount(&doc, "left", "mount-1"));
    assert!(!stores_mount(&doc, "right", "mount-1"));
}

#[test]
fn linking_an_already_paired_instance_errors_and_mutates_nothing() {
    let mut doc = linked_document();
    let before = doc.clone();
    let error = apply(
        &mut doc,
        &[],
        &link_edit(board_target(), "pair-second", "left"),
    )
    .unwrap_err();
    assert!(
        error.contains("already belongs to a mechanical pair"),
        "{error}"
    );
    assert_eq!(doc, before);
}

#[test]
fn unlinking_keeps_geometry_and_localizes_ownership() {
    let mut doc = linked_document();
    // A pair-shared construction edit, as a linked settings change produces.
    doc.hardware.as_mut().unwrap().pairs[0]
        .construction
        .plate_thickness = 2.5;
    let before_right = visible_mount(&doc, "right", "mount-1");
    let before_left = visible_mount(&doc, "left", "mount-1");
    let changed = apply(
        &mut doc,
        &[],
        &lifecycle_edit(MechanicalChange::UnlinkMirrorPair {
            instance_id: "right".into(),
        }),
    )
    .unwrap();
    assert_eq!(changed, vec!["pair-left-right", "left", "right"]);
    assert!(pairs(&doc).is_empty());
    assert_eq!(
        ownership(&doc, "left"),
        MechanicalConstructionOwnership::Local
    );
    assert_eq!(
        ownership(&doc, "right"),
        MechanicalConstructionOwnership::Local
    );
    // The pair construction is materialized, and neither mount moved.
    assert_eq!(stored_instance(&doc, "left").plate_thickness, 2.5);
    assert_eq!(stored_instance(&doc, "right").plate_thickness, 2.5);
    assert_point(
        mount_at(stored_instance(&doc, "right"), "mount-1"),
        before_right,
    );
    assert_point(
        mount_at(stored_instance(&doc, "left"), "mount-1"),
        before_left,
    );
}

#[test]
fn make_local_keeps_visible_geometry_and_leaves_the_other_half_linked() {
    let mut doc = linked_document();
    let changed = apply(
        &mut doc,
        &[],
        &lifecycle_edit(MechanicalChange::MakeFeatureLocal {
            feature_id: "mount-1".into(),
            instance_id: "right".into(),
        }),
    )
    .unwrap();
    // The pair and both member instances are affected, not just the edited half.
    assert_eq!(changed, vec!["mount-1", "right", "pair-left-right", "left"]);
    // The reflected counterpart is materialized on the localized half.
    assert_point(
        mount_at(stored_instance(&doc, "right"), "mount-1"),
        Vec2 { x: -2.0, y: 3.0 },
    );
    let pair = &pairs(&doc)[0];
    assert_eq!(
        features::member_state(pair, "right", "mount-1"),
        Some(MechanicalFeatureState::Local)
    );
    assert_eq!(
        features::member_state(pair, "left", "mount-1"),
        Some(MechanicalFeatureState::Linked)
    );
    assert_eq!(pair.features[0].author, "left");
}

#[test]
fn make_local_on_the_author_moves_authorship_to_the_linked_half() {
    let mut doc = linked_document();
    let changed = apply(
        &mut doc,
        &[],
        &lifecycle_edit(MechanicalChange::MakeFeatureLocal {
            feature_id: "mount-1".into(),
            instance_id: "left".into(),
        }),
    )
    .unwrap();
    assert_eq!(changed, vec!["mount-1", "left", "pair-left-right", "right"]);
    // The author keeps its own visible geometry locally.
    assert_point(
        mount_at(stored_instance(&doc, "left"), "mount-1"),
        Vec2 { x: 2.0, y: 3.0 },
    );
    let pair = &pairs(&doc)[0];
    assert_eq!(
        features::member_state(pair, "left", "mount-1"),
        Some(MechanicalFeatureState::Local)
    );
    assert_eq!(
        features::member_state(pair, "right", "mount-1"),
        Some(MechanicalFeatureState::Linked)
    );
    // Authorship moved, and the stored value is now in the other frame:
    // the reflection of the author's (2, 3) about X at the identity map.
    assert_eq!(pair.features[0].author, "right");
    let MechanicalFeatureValue::Mount { mount } = &pair.features[0].value else {
        panic!("a linked mount stores a mount");
    };
    assert_point(mount.at, Vec2 { x: -2.0, y: 3.0 });
    features::validate(&doc).unwrap();
    // The surviving linked half keeps the geometry it showed, derived from the
    // pair rather than stored on the half.
    assert_point(
        visible_mount(&doc, "right", "mount-1"),
        Vec2 { x: -2.0, y: 3.0 },
    );
    assert!(!stores_mount(&doc, "right", "mount-1"));
}

#[test]
fn omit_removes_only_that_half_and_keeps_the_other_linked() {
    let mut doc = linked_document();
    let changed = apply(
        &mut doc,
        &[],
        &lifecycle_edit(MechanicalChange::OmitFeatureMember {
            feature_id: "mount-1".into(),
            instance_id: "right".into(),
        }),
    )
    .unwrap();
    // The pair and both member instances are affected, not just the edited half.
    assert_eq!(changed, vec!["mount-1", "right", "pair-left-right", "left"]);
    assert!(stored_instance(&doc, "right").mounts.is_empty());
    // The omitted half shows nothing; the still-linked author keeps its geometry.
    assert!(
        !effective_inputs(
            &doc,
            &[],
            &MechanicalTarget::Instance {
                instance_id: "right".into(),
            },
        )
        .unwrap()
        .configuration
        .mounts
        .iter()
        .any(|mount| mount.id == "mount-1"),
        "an omitted half shows no linked geometry"
    );
    assert_point(
        visible_mount(&doc, "left", "mount-1"),
        Vec2 { x: 2.0, y: 3.0 },
    );
    let pair = &pairs(&doc)[0];
    assert_eq!(
        features::member_state(pair, "right", "mount-1"),
        Some(MechanicalFeatureState::Omitted)
    );
    assert_eq!(
        features::member_state(pair, "left", "mount-1"),
        Some(MechanicalFeatureState::Linked)
    );
}

#[test]
fn omit_on_the_author_keeps_the_other_half_deriving_its_geometry() {
    let mut doc = linked_document();
    apply(
        &mut doc,
        &[],
        &lifecycle_edit(MechanicalChange::OmitFeatureMember {
            feature_id: "mount-1".into(),
            instance_id: "left".into(),
        }),
    )
    .unwrap();
    assert!(stored_instance(&doc, "left").mounts.is_empty());
    // The surviving linked half keeps deriving its geometry.
    assert_point(
        visible_mount(&doc, "right", "mount-1"),
        Vec2 { x: -2.0, y: 3.0 },
    );
    let pair = &pairs(&doc)[0];
    assert_eq!(
        features::member_state(pair, "left", "mount-1"),
        Some(MechanicalFeatureState::Omitted)
    );
    assert_eq!(
        features::member_state(pair, "right", "mount-1"),
        Some(MechanicalFeatureState::Linked)
    );
    assert_eq!(pair.features[0].author, "right");
    features::validate(&doc).unwrap();
}

#[test]
fn rejoin_restores_the_link_and_derives_the_reflected_counterpart() {
    let mut doc = linked_document();
    apply(
        &mut doc,
        &[],
        &lifecycle_edit(MechanicalChange::MakeFeatureLocal {
            feature_id: "mount-1".into(),
            instance_id: "right".into(),
        }),
    )
    .unwrap();
    // The localized half has drifted to its own authored position.
    doc.hardware
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.id == "right")
        .unwrap()
        .mechanical
        .as_mut()
        .unwrap()
        .mounts[0]
        .at = Vec2 { x: 100.0, y: 100.0 };

    let changed = apply(
        &mut doc,
        &[],
        &lifecycle_edit(MechanicalChange::RejoinFeatureMember {
            feature_id: "mount-1".into(),
            instance_id: "right".into(),
            source_instance_id: "left".into(),
        }),
    )
    .unwrap();
    assert_eq!(changed, vec!["mount-1", "right", "left", "pair-left-right"]);
    let pair = &pairs(&doc)[0];
    assert_eq!(pair.features[0].author, "left");
    let MechanicalFeatureValue::Mount { mount } = &pair.features[0].value else {
        panic!("a linked mount stores a mount");
    };
    assert_point(mount.at, Vec2 { x: 2.0, y: 3.0 });
    assert_eq!(
        features::member_state(pair, "right", "mount-1"),
        Some(MechanicalFeatureState::Linked)
    );
    // The reflected counterpart replaced the drifted local geometry, and the
    // rejoined half derives it from the pair instead of storing a copy.
    assert_point(
        visible_mount(&doc, "right", "mount-1"),
        Vec2 { x: -2.0, y: 3.0 },
    );
    assert!(!stores_mount(&doc, "right", "mount-1"));
}

#[test]
fn a_no_op_lifecycle_action_returns_an_empty_id_list() {
    let mut doc = linked_document();
    let before = doc.clone();
    assert!(
        apply(
            &mut doc,
            &[],
            &lifecycle_edit(MechanicalChange::RejoinFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
                source_instance_id: "left".into(),
            }),
        )
        .unwrap()
        .is_empty(),
        "an already linked half is unchanged"
    );
    assert_eq!(doc, before);
    doc.hardware.as_mut().unwrap().pairs[0].features[0].members[1].state =
        MechanicalFeatureState::Omitted;
    assert!(
        apply(
            &mut doc,
            &[],
            &lifecycle_edit(MechanicalChange::OmitFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
            }),
        )
        .unwrap()
        .is_empty(),
        "an already omitted half is unchanged"
    );
}

#[test]
fn unknown_pair_feature_and_instance_ids_error_and_mutate_nothing() {
    let mut doc = linked_document();
    let before = doc.clone();
    for change in [
        MechanicalChange::UnlinkMirrorPair {
            instance_id: "ghost".into(),
        },
        MechanicalChange::MakeFeatureLocal {
            feature_id: "ghost".into(),
            instance_id: "right".into(),
        },
        MechanicalChange::OmitFeatureMember {
            feature_id: "mount-1".into(),
            instance_id: "ghost".into(),
        },
        MechanicalChange::RejoinFeatureMember {
            feature_id: "mount-1".into(),
            instance_id: "ghost".into(),
            source_instance_id: "left".into(),
        },
    ] {
        assert!(
            apply(&mut doc, &[], &lifecycle_edit(change.clone())).is_err(),
            "{change:?} must error"
        );
        assert_eq!(doc, before, "{change:?} must mutate nothing");
    }
    let mut missing_template = link_edit(board_target(), "pair-left-right", "left");
    missing_template.mounting_hole = None;
    assert!(
        apply(&mut doc, &[], &missing_template).is_err(),
        "the projection template is required"
    );
    assert_eq!(doc, before);
}
