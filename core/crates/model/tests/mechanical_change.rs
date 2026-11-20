use boardstudio_model::mechanical::change::*;
use boardstudio_model::model::*;
use serde_json::json;

#[test]
fn a_mechanical_edit_round_trips_through_its_wire_shape() {
    let edit = MechanicalEdit {
        target: MechanicalTarget::Instance {
            instance_id: "left".into(),
        },
        change: MechanicalChange::SetDimension {
            field: MechanicalDimension::PlateThickness,
            value: 1.5,
        },
        mounting_hole: None,
    };
    let value = serde_json::to_value(&edit).unwrap();
    assert_eq!(value["target"]["kind"], "instance");
    assert_eq!(value["target"]["instanceId"], "left");
    // The change vocabulary is externally tagged: one key names the change.
    assert_eq!(value["change"]["set-dimension"]["field"], "plate-thickness");
    assert_eq!(value["change"]["set-dimension"]["value"], 1.5);
    assert!(value.get("mountingHole").is_none());
    let parsed: MechanicalEdit = serde_json::from_value(value).unwrap();
    assert_eq!(parsed, edit);
}

#[test]
fn a_mechanical_edit_serializes_inside_an_edit_operation() {
    let operation = boardstudio_model::model::EditOperation::ApplyMechanicalEdit {
        edit: Box::new(MechanicalEdit {
            target: MechanicalTarget::Board {
                board_id: "board".into(),
            },
            change: MechanicalChange::AddMount {
                collection: MechanicalMountCollection::Closure,
            },
            mounting_hole: None,
        }),
    };
    let value = serde_json::to_value(&operation).unwrap();
    assert_eq!(value["kind"], "apply-mechanical-edit");
    assert_eq!(value["edit"]["target"]["kind"], "board");
    assert_eq!(
        value["edit"]["change"]["add-mount"]["collection"],
        "closure"
    );
    let parsed: boardstudio_model::model::EditOperation = serde_json::from_value(value).unwrap();
    assert_eq!(parsed, operation);
}

#[test]
fn the_link_lifecycle_vocabulary_round_trips_through_its_wire_shape() {
    let correspondence = MechanicalFeatureCorrespondence {
        kind: MechanicalFeatureKind::Mount,
        first_feature_id: "mount-1".into(),
        second_feature_id: "mount-1".into(),
        source_instance_id: Some("left".into()),
    };
    let link = MechanicalChange::LinkMirrorPair {
        pair_id: "pair-left-right".into(),
        members: ["left".into(), "right".into()],
        mapping: Pose2 {
            at: Vec2 { x: 10.0, y: 5.0 },
            rotation: 90.0,
        },
        construction_source: "left".into(),
        correspondences: vec![correspondence],
    };
    let value = serde_json::to_value(&link).unwrap();
    // Externally tagged and kebab-case, like every other change.
    let payload = &value["link-mirror-pair"];
    assert_eq!(payload["pair_id"], json!("pair-left-right"));
    assert_eq!(payload["members"], json!(["left", "right"]));
    assert_eq!(payload["mapping"]["rotation"], json!(90.0));
    assert_eq!(payload["construction_source"], json!("left"));
    // The correspondence is a standalone struct, so its own fields are camelCase.
    assert_eq!(payload["correspondences"][0]["kind"], json!("mount"));
    assert_eq!(
        payload["correspondences"][0]["firstFeatureId"],
        json!("mount-1")
    );
    assert_eq!(
        payload["correspondences"][0]["secondFeatureId"],
        json!("mount-1")
    );
    assert_eq!(
        payload["correspondences"][0]["sourceInstanceId"],
        json!("left")
    );
    let parsed: MechanicalChange = serde_json::from_value(value).unwrap();
    assert_eq!(parsed, link);

    let lifecycle = vec![
        MechanicalChange::UnlinkMirrorPair {
            instance_id: "left".into(),
        },
        MechanicalChange::MakeFeatureLocal {
            feature_id: "mount-1".into(),
            instance_id: "left".into(),
        },
        MechanicalChange::OmitFeatureMember {
            feature_id: "mount-1".into(),
            instance_id: "left".into(),
        },
        MechanicalChange::RejoinFeatureMember {
            feature_id: "mount-1".into(),
            instance_id: "left".into(),
            source_instance_id: "right".into(),
        },
    ];
    let keys = [
        "unlink-mirror-pair",
        "make-feature-local",
        "omit-feature-member",
        "rejoin-feature-member",
    ];
    for (change, key) in lifecycle.into_iter().zip(keys) {
        let value = serde_json::to_value(&change).unwrap();
        assert!(value.get(key).is_some(), "{key} tags {change:?}");
        let parsed: MechanicalChange = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, change);
    }

    // A unit change is the bare externally-tagged key, exactly like Enable.
    assert_eq!(
        serde_json::to_value(MechanicalChange::RemovePhysicalInstance).unwrap(),
        json!("remove-physical-instance")
    );
    assert_eq!(
        serde_json::from_value::<MechanicalChange>(json!("remove-physical-instance")).unwrap(),
        MechanicalChange::RemovePhysicalInstance
    );
}

#[test]
fn the_opening_vocabulary_addresses_durable_ids() {
    let changes = vec![
        (
            MechanicalChange::RemoveOpening {
                opening_id: "opening-2".into(),
            },
            "remove-opening",
        ),
        (
            MechanicalChange::AddOpeningVertex {
                opening_id: "opening-2".into(),
            },
            "add-opening-vertex",
        ),
        (
            MechanicalChange::RemoveOpeningVertex {
                opening_id: "opening-2".into(),
                vertex_id: "vertex-3".into(),
            },
            "remove-opening-vertex",
        ),
        (
            MechanicalChange::SetOpeningDimension {
                opening_id: "opening-2".into(),
                vertex_id: Some("vertex-3".into()),
                field: MechanicalDimension::OpeningPointX,
                value: 4.5,
            },
            "set-opening-dimension",
        ),
        (
            MechanicalChange::SetOpeningDimension {
                opening_id: "opening-2".into(),
                vertex_id: None,
                field: MechanicalDimension::OpeningHeight,
                value: 9.0,
            },
            "set-opening-dimension",
        ),
    ];
    // Positional identities are gone: a queued or linked opening edit names
    // only durable opening/vertex ids, never a vector index.
    for (change, key) in changes {
        let value = serde_json::to_value(&change).unwrap();
        let payload = value
            .get(key)
            .unwrap_or_else(|| panic!("{key} tags {change:?}"));
        assert_eq!(payload["opening_id"], json!("opening-2"), "{change:?}");
        assert!(payload.get("opening_index").is_none(), "{change:?}");
        assert!(payload.get("point_index").is_none(), "{change:?}");
        let parsed: MechanicalChange = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, change);
    }
}

#[test]
fn a_mounting_hole_definition_round_trips_when_present() {
    let edit = MechanicalEdit {
        target: MechanicalTarget::Board {
            board_id: "board".into(),
        },
        change: MechanicalChange::InitializeClosures,
        mounting_hole: Some(Box::new(
            serde_json::from_value(json!({
                "id": "ceoloide/mounting_hole_npth",
                "name": "Mounting hole",
                "kind": "utility",
                "courtyard": [],
                "pads": []
            }))
            .unwrap(),
        )),
    };
    let value = serde_json::to_value(&edit).unwrap();
    assert_eq!(
        value["mountingHole"]["id"],
        json!("ceoloide/mounting_hole_npth")
    );
    let parsed: MechanicalEdit = serde_json::from_value(value).unwrap();
    assert_eq!(parsed, edit);
}
