//! Typed mechanical change vocabulary and the edit that carries it.
//!
//! The payload names the specific user change; defaults, effective settings,
//! feature ownership and generated clearance parts stay behind the mechanical
//! module's seam. UI field IDs, labels and drafts live in the Case adapter.
use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalMountCollection {
    Suspension,
    Closure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalDimension {
    PlateThickness,
    PlateFoamThickness,
    PcbThickness,
    BottomFoamThickness,
    BottomThickness,
    WallThickness,
    Clearance,
    GasketSupportLength,
    GasketSupportWidth,
    OpeningAllowance,
    OpeningBottomZ,
    OpeningHeight,
    OpeningPointX,
    OpeningPointY,
    BatteryWidth,
    BatteryDepth,
    BatteryHeight,
    BatteryCableWidth,
    BatteryPositionX,
    BatteryPositionY,
    BatteryCableExitX,
    BatteryCableExitY,
    MountPositionX,
    MountPositionY,
    MountHoleDiameter,
    MountBossDiameter,
    MountBossHeight,
    ClosureThreadDiameter,
    ClosurePitch,
    ClosureHeadDiameter,
    ClosureHeadHeight,
    ClosureHoleDiameter,
    ClosureInsertDiameter,
    ClosureInsertLength,
    ClosureSeatDiameter,
    ClosureSeatDepth,
    ClosureEngagement,
    ClosureThreadStart,
    ClosureTipAllowance,
    ClosureBottomingClearance,
    ClosureRoof,
    ClosureSurround,
    ClosureSeatLeadDepth,
    ClosureSeatLeadDiameter,
    ClosureBearingThickness,
    CriticalFitFromX,
    CriticalFitFromY,
    CriticalFitToX,
    CriticalFitToY,
    HardwareLength,
    HardwareQuantity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalCriticalFitTextField {
    Label,
    Tolerance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalHardwareTextField {
    Designation,
    Thread,
    Tolerance,
    Notes,
}

/// One reviewed match between the two members of a mechanical mirror pair.
///
/// `first_feature_id` names the feature on `members[0]` and `second_feature_id`
/// the matching feature on `members[1]` that the link replaces; both must exist
/// with the given kind. Linking consolidates the two local features into one
/// shared value authored by the reviewed `source_instance_id` (or `members[0]`
/// when absent). The relationship keeps the authored value's id on both halves.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalFeatureCorrespondence {
    pub kind: MechanicalFeatureKind,
    pub first_feature_id: String,
    pub second_feature_id: String,
    /// The half whose accepted geometry becomes the shared authored value for this
    /// feature. `None` keeps the legacy behaviour of sourcing from `members[0]`.
    /// The choice is per feature and independent of the pair's construction source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_instance_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MechanicalChange {
    Enable,
    InitializeClosures,
    Disable,
    SetBatteryEnabled(bool),
    SetClosureInsertPreset(String),
    SetClosureDrive(ScrewDrive),
    SetClosureInstallation(InsertInstallation),
    SetClosureFixedLength(Option<f64>),
    SetClosureThread(String),
    SetClosureScrewLengths(Vec<f64>),
    SetClosureHeadProfile(ScrewHeadProfile),
    SetClosureLengthDatum(ScrewLengthDatum),
    AddCriticalFit {
        part_id: String,
    },
    RemoveCriticalFit {
        fit_id: String,
    },
    SetCriticalFitText {
        fit_id: String,
        field: MechanicalCriticalFitTextField,
        value: String,
    },
    SetCriticalFitPart {
        fit_id: String,
        part_id: String,
    },
    SetCriticalFitPoint {
        fit_id: String,
        field: MechanicalDimension,
        value: f64,
    },
    AddHardware {
        part_id: String,
        feature_id: String,
    },
    RemoveHardware {
        hardware_id: String,
    },
    SetHardwareMount {
        hardware_id: String,
        part_id: String,
        feature_id: String,
    },
    SetHardwareText {
        hardware_id: String,
        field: MechanicalHardwareTextField,
        value: String,
    },
    SetHardwareDimension {
        hardware_id: String,
        field: MechanicalDimension,
        value: f64,
    },
    AddOpening,
    RemoveOpening {
        opening_id: String,
    },
    AddOpeningVertex {
        opening_id: String,
    },
    RemoveOpeningVertex {
        opening_id: String,
        vertex_id: String,
    },
    SetOpeningDimension {
        opening_id: String,
        vertex_id: Option<String>,
        field: MechanicalDimension,
        value: f64,
    },
    SetMethod(PlateMethod),
    SetMount(MechanicalMount),
    AddMount {
        collection: MechanicalMountCollection,
    },
    AddSuggestedClosureMounts(Vec<Mount>),
    SetMountCollection {
        collection: MechanicalMountCollection,
        mounts: Vec<Mount>,
    },
    SetMountDimension {
        collection: MechanicalMountCollection,
        mount_id: String,
        field: MechanicalDimension,
        value: f64,
    },
    SetMountPosition {
        collection: MechanicalMountCollection,
        mount_id: String,
        at: Vec2,
    },
    SetMountKind {
        collection: MechanicalMountCollection,
        mount_id: String,
        kind: MountKind,
    },
    RemoveMount {
        collection: MechanicalMountCollection,
        mount_id: String,
    },
    AdoptClosurePositions(Vec<Mount>),
    SetBottomStyle(MechanicalBottomStyle),
    SetMiddleFrame(bool),
    SetIntegratedPlateFrame(bool),
    SetDimension {
        field: MechanicalDimension,
        value: f64,
    },
    SetGasketSupportDimension {
        support_id: String,
        anchors: Vec<MechanicalGasketAnchor>,
        field: MechanicalDimension,
        value: f64,
    },
    SetGasketSupportPlacement {
        support_id: String,
        anchors: Vec<MechanicalGasketAnchor>,
    },
    SetGasketSupportUnlinked {
        support_id: String,
        pair_id: Option<String>,
        anchors: Vec<MechanicalGasketAnchor>,
    },
    ResetGasketPlacement,
    AssignSwitchProfile {
        definition_id: String,
        family: MechanicalSwitchFamily,
    },
    AssignStabilizerProfile {
        definition_id: String,
        source: MechanicalBuiltinProfile,
    },
    AssignImportedGeometryProfile {
        definition_id: String,
    },
    UpdateProfile(MechanicalPartProfile),
    RemoveProfile {
        definition_id: String,
    },
    SetSwitchFamily {
        definition_id: String,
        family: MechanicalSwitchFamily,
    },
    AddProcessOverride {
        part_id: String,
    },
    RemoveProcessOverride {
        part_id: String,
    },
    SetProcessMethod {
        part_id: String,
        method: PlateMethod,
    },
    SetProcessMaterial {
        part_id: String,
        material: String,
    },
    SetProcessThickness {
        part_id: String,
        thickness: f64,
    },
    SetStabilizerKind(MechanicalStabilizerOverride),
    /// Establishes an explicit mechanical mirror pair between two physical
    /// halves: the shared construction comes from `construction_source`, and
    /// each correspondence becomes one linked feature authored by `members[0]`.
    LinkMirrorPair {
        pair_id: String,
        members: [String; 2],
        mapping: Pose2,
        construction_source: String,
        correspondences: Vec<MechanicalFeatureCorrespondence>,
    },
    /// Dissolves the pair that owns `instance_id`, materializing the pair's
    /// construction and linked geometry locally so nothing moves.
    UnlinkMirrorPair {
        instance_id: String,
    },
    /// Removes one physical half from its explicit mechanical pair. The surviving
    /// half keeps the geometry, construction and hardware it currently shows,
    /// materialized locally, and the pair is cleared. The target must be the
    /// removed half's instance; a board target is malformed.
    RemovePhysicalInstance,
    /// Converts selected legacy-ownership physical instances to local ownership.
    ///
    /// Each named member's current effective values are materialized into its own
    /// configuration - including the overlaid hardware, critical fits and profiles -
    /// before its ownership becomes `Local`, so nothing it showed changes. When the
    /// remaining legacy members' shared source is one of the converted members, the
    /// prior common construction is captured as the legacy source first, so their
    /// baseline survives conversion and later edits to the converted member cannot
    /// feed back into it.
    MaterializeLegacyConstruction {
        instance_ids: Vec<String>,
    },
    /// Makes one linked feature local on one half without moving its visible
    /// geometry; the other half and every other feature keep their link.
    MakeFeatureLocal {
        feature_id: String,
        instance_id: String,
    },
    /// Removes one linked feature from one half only (a local omission); the
    /// other half and every other feature keep their link.
    OmitFeatureMember {
        feature_id: String,
        instance_id: String,
    },
    /// Clears one half's local/omitted exception, taking `source_instance_id`'s
    /// geometry as the authored value and re-deriving the counterpart.
    RejoinFeatureMember {
        feature_id: String,
        instance_id: String,
        source_instance_id: String,
    },
}

/// The board or physical half a mechanical edit targets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MechanicalTarget {
    /// The canonical project-level mechanical configuration.
    Board {
        #[serde(rename = "boardId")]
        board_id: String,
    },
    /// One physical half's configuration.
    Instance {
        #[serde(rename = "instanceId")]
        instance_id: String,
    },
}

/// One typed mechanical edit: a typed target, a typed change and the optional
/// normalized mounting-hole catalogue definition projection needs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalEdit {
    pub target: MechanicalTarget,
    pub change: MechanicalChange,
    /// Supplied whenever projection must create or update a generated
    /// closure-hole footprint. Validated against its declared source identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mounting_hole: Option<Box<PartDefinition>>,
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let operation = crate::model::EditOperation::ApplyMechanicalEdit {
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
        let parsed: crate::model::EditOperation = serde_json::from_value(value).unwrap();
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
}
