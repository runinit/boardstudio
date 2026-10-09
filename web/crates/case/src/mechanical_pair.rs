//! The Case view of a physical half's mechanical mirror pair, and the reviewed-input
//! fingerprint its per-feature actions are submitted with.
//!
//! Every row is derived from the accepted pair store, so there is no second mutable
//! relationship store. The fingerprint covers exactly the inputs a reviewed link or
//! rejoin depends on - membership, mapping, the chosen feature identity, its kind and
//! the value the result is derived from, plus the pair's shared construction when a
//! link consolidates it - so unrelated edits and revision changes leave a review valid
//! while a relevant input change retires it.
use boardstudio_core::mechanical::MechanicalTarget;
use boardstudio_core::mechanical::change::{MechanicalChange, MechanicalFeatureCorrespondence};
use boardstudio_core::mechanical::features;
use boardstudio_core::mechanical::inputs::effective_inputs;
use boardstudio_core::model::{
    BoardContours, MechanicalConfiguration, MechanicalFeature, MechanicalFeatureKind,
    MechanicalFeatureMember, MechanicalFeatureState, MechanicalFeatureValue, MechanicalMirrorPair,
    Pose2, ProjectDoc, Vec2,
};

/// One linked feature of the selected half's pair, as the accepted document stores it.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalPairFeatureRow {
    /// The durable feature id of this half's member record.
    pub feature_id: String,
    pub kind: MechanicalFeatureKind,
    /// This half's accepted state: Linked, Local or Omitted.
    pub state: MechanicalFeatureState,
    /// The half this row was projected for.
    pub instance_id: String,
    /// The other half of the pair.
    pub counterpart_instance_id: String,
    /// The other half's durable feature id for the same linked feature.
    pub counterpart_feature_id: String,
    /// The geometry a rejoin would take from the counterpart, described for review.
    /// `None` when the counterpart owns no linked value to take.
    pub preview: Option<String>,
    /// The fingerprint of the accepted inputs this row's review was produced from. The
    /// rejoin submission carries it, so a relevant change between the displayed review
    /// and its execution retires the action instead of applying newer state.
    /// `None` when the reviewed inputs cannot be fingerprinted, which also disables it.
    pub rejoin_fingerprint: Option<String>,
}

/// The selected half's pair, with one row per linked feature.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalPairProjection {
    pub pair_id: String,
    pub members: [String; 2],
    pub features: Vec<MechanicalPairFeatureRow>,
}

/// The selected half's pair projection, or `None` when it belongs to no pair.
pub fn pair_projection(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    instance_id: &str,
) -> Option<MechanicalPairProjection> {
    let hardware = document.hardware.as_ref()?;
    let pair = hardware
        .pairs
        .iter()
        .find(|pair| pair.members.iter().any(|member| member == instance_id))?;
    let counterpart = pair
        .members
        .iter()
        .find(|member| member.as_str() != instance_id)?;
    let features = pair
        .features
        .iter()
        .filter_map(|feature| {
            let member = member_record(feature, instance_id)?;
            let counterpart_member = member_record(feature, counterpart)?;
            Some(MechanicalPairFeatureRow {
                feature_id: member.feature_id.clone(),
                kind: feature.kind,
                state: member.state,
                instance_id: instance_id.to_owned(),
                counterpart_instance_id: counterpart.clone(),
                counterpart_feature_id: counterpart_member.feature_id.clone(),
                preview: rejoin_preview(document, board_contours, pair, feature, counterpart),
                rejoin_fingerprint: reviewed_fingerprint(
                    document,
                    board_contours,
                    &MechanicalChange::RejoinFeatureMember {
                        feature_id: member.feature_id.clone(),
                        instance_id: instance_id.to_owned(),
                        source_instance_id: counterpart.clone(),
                    },
                )
                .ok(),
            })
        })
        .collect();
    Some(MechanicalPairProjection {
        pair_id: pair.id.clone(),
        members: pair.members.clone(),
        features,
    })
}

/// One candidate link between the selected half and another physical instance.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalPairLinkOption {
    pub pair_id: String,
    /// The selected half.
    pub instance_id: String,
    pub other_instance_id: String,
    pub other_name: String,
    /// The map the halves' declared transforms imply. `None` means the review must
    /// supply one for a translated or rotated distinct board frame.
    pub derived_mapping: Option<Pose2>,
    /// The construction source the review starts from.
    pub construction_source: String,
    /// The durable-id feature matches the link would consolidate. Features without a
    /// match stay local on their half.
    pub correspondences: Vec<MechanicalFeatureCorrespondence>,
    /// A short description of those matches, for the review.
    pub matched_features: String,
    /// The fingerprint of the accepted inputs this candidate's review was produced
    /// from. The link submission carries it for the same reason as a rejoin row's.
    pub reviewed_fingerprint: Option<String>,
}

/// The candidate links the selected half can establish, from the accepted document.
///
/// Empty when the half already belongs to a pair or the document has no other
/// instance. Each option is the reviewable content of the link; establishing it is a
/// separate, reviewed submission.
pub fn pair_link_options(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    instance_id: &str,
) -> Vec<MechanicalPairLinkOption> {
    let Some(hardware) = document.hardware.as_ref() else {
        return Vec::new();
    };
    if hardware
        .pairs
        .iter()
        .any(|pair| pair.members.iter().any(|member| member == instance_id))
    {
        return Vec::new();
    }
    hardware
        .instances
        .iter()
        .filter(|other| other.id != instance_id)
        .filter_map(|other| {
            let pair_id = format!("pair/{instance_id}");
            let review = boardstudio_web_catalogue::physical_setup::review_mirror_pair_link(
                document,
                &pair_id,
                instance_id,
                &other.id,
            )
            .ok()
            .flatten()?;
            let reviewed = MechanicalChange::LinkMirrorPair {
                pair_id: pair_id.clone(),
                members: [instance_id.to_owned(), other.id.clone()],
                // The reviewed map and construction source are the user's payload and
                // are excluded from the fingerprint, so this offer only needs a
                // well-formed placeholder for them.
                mapping: review.derived_mapping.unwrap_or(Pose2 {
                    at: Vec2::default(),
                    rotation: 0.0,
                }),
                construction_source: review.construction_source.clone(),
                correspondences: review.correspondences.clone(),
            };
            Some(MechanicalPairLinkOption {
                pair_id,
                instance_id: instance_id.to_owned(),
                other_instance_id: other.id.clone(),
                other_name: other.name.clone(),
                derived_mapping: review.derived_mapping,
                construction_source: review.construction_source,
                matched_features: matched_features(&review.correspondences),
                correspondences: review.correspondences,
                reviewed_fingerprint: reviewed_fingerprint(document, board_contours, &reviewed)
                    .ok(),
            })
        })
        .collect()
}

/// A short description of the durable-id matches a reviewed link would consolidate.
fn matched_features(correspondences: &[MechanicalFeatureCorrespondence]) -> String {
    if correspondences.is_empty() {
        return "No shared mechanical features yet.".to_owned();
    }
    correspondences
        .iter()
        .map(|correspondence| {
            format!(
                "{} {}",
                feature_label(correspondence.kind),
                correspondence.first_feature_id
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// What a rejoin would take from `source_instance_id`, for the review preview.
///
/// The source half's visible geometry is read through Core's effective-input path, so
/// the preview describes exactly the value the rejoin will install rather than a
/// second copy of Core's linked-value rules. `pair` is unused beyond documenting the
/// relationship the two halves share.
fn rejoin_preview(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    _pair: &MechanicalMirrorPair,
    feature: &MechanicalFeature,
    source_instance_id: &str,
) -> Option<String> {
    let source = member_record(feature, source_instance_id)?;
    if source.state == MechanicalFeatureState::Omitted {
        return None;
    }
    let inputs = effective_inputs(
        document,
        board_contours,
        &MechanicalTarget::Instance {
            instance_id: source_instance_id.to_owned(),
        },
    )
    .ok()?;
    configuration_feature(&inputs.configuration, feature.kind, &source.feature_id)
        .map(|value| describe_value(feature.kind, &value))
}

/// A short human-readable description of the geometry a feature carries.
fn describe_value(kind: MechanicalFeatureKind, value: &MechanicalFeatureValue) -> String {
    let label = feature_label(kind);
    match value {
        MechanicalFeatureValue::Mount { mount } => {
            format!("{label} at ({:?}, {:?})", mount.at.x, mount.at.y)
        }
        MechanicalFeatureValue::Opening { opening } => {
            format!("{label} with {} vertices", opening.vertices.len())
        }
        MechanicalFeatureValue::GasketSupport { anchor } => {
            format!("{label} at {:?} along its region", anchor.anchor)
        }
    }
}

/// The display name of one linked feature kind.
pub fn feature_label(kind: MechanicalFeatureKind) -> &'static str {
    match kind {
        MechanicalFeatureKind::Mount => "Mount",
        MechanicalFeatureKind::ClosureMount => "Closure mount",
        MechanicalFeatureKind::Opening => "Opening",
        MechanicalFeatureKind::GasketSupport => "Gasket support",
    }
}

/// The fingerprint of the reviewed inputs a link or rejoin submission depends on.
///
/// It is recomputed from the accepted document when Session resolves the submission;
/// a different value means a relevant input changed since the review, so the reviewed
/// action must retire and the user must review the refreshed result. `Err` names an
/// input that is already gone or malformed, which is also a retirement: the reviewed
/// action can no longer describe the accepted document.
pub fn reviewed_fingerprint(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    change: &MechanicalChange,
) -> Result<String, String> {
    match change {
        MechanicalChange::RejoinFeatureMember {
            feature_id,
            instance_id,
            ..
        } => rejoin_fingerprint(document, feature_id, instance_id),
        MechanicalChange::LinkMirrorPair {
            pair_id,
            members,
            correspondences,
            ..
        } => link_fingerprint(document, board_contours, pair_id, members, correspondences),
        _ => Err("The reviewed-input fingerprint only covers link and rejoin actions.".into()),
    }
}

/// The rejoin inputs the accepted document contributes: the pair's membership and map,
/// the feature's authored value with its author, and both halves' feature ids and
/// states. The half the user picked as the source is the reviewed payload and is
/// excluded, so choosing a source after the review was produced is not staleness.
fn rejoin_fingerprint(
    document: &ProjectDoc,
    feature_id: &str,
    instance_id: &str,
) -> Result<String, String> {
    let hardware = document
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?;
    let pair = hardware
        .pairs
        .iter()
        .find(|pair| located_feature(pair, instance_id, feature_id).is_some())
        .ok_or_else(|| {
            format!("Physical instance {instance_id:?} has no linked feature {feature_id:?}.")
        })?;
    let feature = located_feature(pair, instance_id, feature_id).ok_or_else(|| {
        format!(
            "The mechanical pair {:?} has no feature {feature_id:?}.",
            pair.id
        )
    })?;
    let destination = member_record(feature, instance_id).ok_or_else(|| {
        format!("Physical instance {instance_id:?} has no linked feature {feature_id:?}.")
    })?;
    let counterpart = pair
        .members
        .iter()
        .find(|member| member.as_str() != instance_id)
        .ok_or("The mechanical pair has no other member.")?;
    let counterpart_record = member_record(feature, counterpart).ok_or_else(|| {
        format!(
            "The mechanical pair {:?} has no counterpart feature.",
            pair.id
        )
    })?;
    let mut parts = vec![
        "rejoin".to_owned(),
        quoted(&pair.id),
        quoted(&pair.members[0]),
        quoted(&pair.members[1]),
        pose(&pair.mapping),
        quoted(&feature.id),
        format!("{:?}", feature.kind),
        quoted(&feature.author),
        encoded(&feature.value)?,
        quoted(instance_id),
        format!("{:?}", destination.state),
        quoted(&destination.feature_id),
        quoted(&counterpart_record.feature_id),
        format!("{:?}", counterpart_record.state),
    ];
    parts.shrink_to_fit();
    Ok(parts.join("|"))
}

/// The link inputs the accepted document contributes to a reviewed link.
///
/// This covers exactly the document-derived inputs the reviewed result rests on: the
/// pair id and members, each half's declared transform (which is what a derived map
/// comes from), both halves' shared construction and each matched feature's geometry
/// on both halves. The reviewed *choices* - the map the user reviewed or typed, which
/// half supplies construction, and (once Core carries it) which half supplies each
/// matched feature - are the payload the user confirmed and travel with the
/// submission, so changing them is not document staleness and does not change this
/// fingerprint.
fn link_fingerprint(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    pair_id: &str,
    members: &[String; 2],
    correspondences: &[boardstudio_core::mechanical::change::MechanicalFeatureCorrespondence],
) -> Result<String, String> {
    let hardware = document
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?;
    if hardware.pairs.iter().any(|pair| pair.id == pair_id) {
        return Err(format!(
            "A mechanical pair named {pair_id:?} already exists."
        ));
    }
    let declared = |instance_id: &str| {
        hardware
            .instances
            .iter()
            .find(|instance| instance.id == instance_id)
            .map(|instance| instance.flipped)
            .ok_or_else(|| format!("Physical instance {instance_id:?} is unavailable."))
    };
    let configuration = |instance_id: &str| {
        effective_inputs(
            document,
            board_contours,
            &MechanicalTarget::Instance {
                instance_id: instance_id.to_owned(),
            },
        )
        .map(|inputs| inputs.configuration)
    };
    let first_flipped = declared(&members[0])?;
    let second_flipped = declared(&members[1])?;
    let first = configuration(&members[0])?;
    let second = configuration(&members[1])?;
    let mut matched = Vec::with_capacity(correspondences.len());
    for correspondence in correspondences {
        let first_value = feature_geometry(
            &first,
            correspondence.kind,
            &correspondence.first_feature_id,
        )
        .ok_or_else(|| {
            format!(
                "Physical instance {:?} has no {:?} feature {:?}.",
                members[0], correspondence.kind, correspondence.first_feature_id
            )
        })?;
        let second_value = feature_geometry(
            &second,
            correspondence.kind,
            &correspondence.second_feature_id,
        )
        .ok_or_else(|| {
            format!(
                "Physical instance {:?} has no {:?} feature {:?}.",
                members[1], correspondence.kind, correspondence.second_feature_id
            )
        })?;
        matched.push(format!(
            "{:?}:{}:{}:{first_value}:{second_value}",
            correspondence.kind,
            quoted(&correspondence.first_feature_id),
            quoted(&correspondence.second_feature_id),
        ));
    }
    Ok([
        "link".to_owned(),
        quoted(pair_id),
        quoted(&members[0]),
        quoted(&members[1]),
        format!("flipped:{first_flipped}:{second_flipped}"),
        encoded(&features::shared_construction(&first))?,
        encoded(&features::shared_construction(&second))?,
        matched.join(","),
    ]
    .join("|"))
}

/// The linked feature that owns one half's durable member feature id.
fn located_feature<'a>(
    pair: &'a MechanicalMirrorPair,
    instance_id: &str,
    feature_id: &str,
) -> Option<&'a MechanicalFeature> {
    pair.features.iter().find(|feature| {
        member_record(feature, instance_id).is_some_and(|member| member.feature_id == feature_id)
    })
}

/// The member record a linked feature has for one half, by durable id.
fn member_record<'a>(
    feature: &'a MechanicalFeature,
    instance_id: &str,
) -> Option<&'a MechanicalFeatureMember> {
    feature
        .members
        .iter()
        .find(|member| member.instance_id == instance_id)
}

/// One feature of a half's effective configuration, by its durable id.
fn configuration_feature(
    configuration: &MechanicalConfiguration,
    kind: MechanicalFeatureKind,
    feature_id: &str,
) -> Option<MechanicalFeatureValue> {
    match kind {
        MechanicalFeatureKind::Mount => configuration
            .mounts
            .iter()
            .find(|mount| mount.id == feature_id)
            .cloned()
            .map(|mount| MechanicalFeatureValue::Mount { mount }),
        MechanicalFeatureKind::ClosureMount => configuration
            .closure_mounts
            .as_ref()?
            .iter()
            .find(|mount| mount.id == feature_id)
            .cloned()
            .map(|mount| MechanicalFeatureValue::Mount { mount }),
        MechanicalFeatureKind::Opening => configuration
            .openings
            .as_ref()?
            .iter()
            .find(|opening| opening.id == feature_id)
            .cloned()
            .map(|opening| MechanicalFeatureValue::Opening { opening }),
        MechanicalFeatureKind::GasketSupport => configuration
            .gasket_layout
            .as_ref()?
            .supports
            .iter()
            .find(|support| support.id == feature_id)
            .cloned()
            .map(|anchor| MechanicalFeatureValue::GasketSupport { anchor }),
    }
}

fn feature_geometry(
    configuration: &MechanicalConfiguration,
    kind: MechanicalFeatureKind,
    feature_id: &str,
) -> Option<String> {
    configuration_feature(configuration, kind, feature_id).and_then(|value| encoded(&value).ok())
}

fn encoded<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| error.to_string())
}

fn quoted(value: &str) -> String {
    encoded(&value).unwrap_or_else(|_| value.to_owned())
}

fn pose(mapping: &Pose2) -> String {
    format!(
        "{:?}:{:?}:{:?}",
        mapping.at.x, mapping.at.y, mapping.rotation
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{
        Board, GasketPlacement, HardwareConfiguration, HardwareTopology, HardwareTransport,
        MechanicalConstructionOwnership, MechanicalFeatureValue, MechanicalGasketAnchor,
        MechanicalGasketLayout, MechanicalMirrorPair, Mount, MountKind, PhysicalBoardInstance,
        Vec2,
    };
    use serde_json::json;

    fn configuration(board_id: &str) -> MechanicalConfiguration {
        serde_json::from_value(json!({
            "boardId": board_id, "method":"printed", "mount":"tray",
            "integratedPlateFrame":false, "bottomStyle":"shell", "middleFrame":false,
            "plateThickness":1.5, "plateFoamThickness":3.0, "pcbThickness":1.6,
            "bottomFoamThickness":2.0, "batteryHeight":0.0, "bottomThickness":3.0,
            "plateToPcb":3.5, "wallThickness":2.0, "clearance":0.3,
            "mounts":[], "closureMounts":[], "profiles":[]
        }))
        .unwrap()
    }

    fn instance(id: &str, board_id: &str) -> PhysicalBoardInstance {
        PhysicalBoardInstance {
            id: id.into(),
            name: id.into(),
            board_id: board_id.into(),
            half: id.into(),
            role: "custom-role".into(),
            flipped: false,
            controller_part_id: None,
            mechanical: Some(configuration(board_id)),
            construction_ownership: MechanicalConstructionOwnership::Pair,
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

    fn anchor(id: &str, fraction: f64) -> MechanicalGasketAnchor {
        MechanicalGasketAnchor {
            length: None,
            width: None,
            placement: Some(GasketPlacement::Generated),
            id: id.into(),
            region_id: "region-1".into(),
            outline_key: "outline-1".into(),
            anchor: fraction,
            unlinked: false,
        }
    }

    fn gasket_layout(supports: Vec<MechanicalGasketAnchor>) -> MechanicalGasketLayout {
        MechanicalGasketLayout {
            auto_size: None,
            adhesive_thickness: None,
            minimum_foam_thickness: None,
            preset_id: None,
            material: None,
            length: 80.0,
            width: 3.0,
            thickness: 2.0,
            compression: 0.15,
            supports,
        }
    }

    fn linked_feature(
        id: &str,
        kind: MechanicalFeatureKind,
        value: MechanicalFeatureValue,
        first_state: MechanicalFeatureState,
        second_state: MechanicalFeatureState,
    ) -> MechanicalFeature {
        MechanicalFeature {
            id: id.into(),
            kind,
            author: "left".into(),
            value,
            members: vec![
                MechanicalFeatureMember {
                    instance_id: "left".into(),
                    feature_id: format!("{id}-left"),
                    state: first_state,
                },
                MechanicalFeatureMember {
                    instance_id: "right".into(),
                    feature_id: format!("{id}-right"),
                    state: second_state,
                },
            ],
        }
    }

    /// One pair with a local mount, a linked opening and an omitted gasket support.
    fn document() -> ProjectDoc {
        let mut doc = ProjectDoc::empty("doc", "Fixture");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let mut left = configuration("board");
        left.mounts = vec![mount("mount-1-left", 2.0, 3.0)];
        left.openings = Some(vec![]);
        left.gasket_layout = Some(gasket_layout(vec![anchor("support-1-left", 0.25)]));
        let mut right = configuration("board");
        right.mounts = vec![mount("mount-1-right", -2.0, 3.0)];
        right.gasket_layout = Some(gasket_layout(vec![anchor("support-1-right", 0.25)]));
        let pair = MechanicalMirrorPair {
            id: "pair-1".into(),
            members: ["left".into(), "right".into()],
            mapping: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            construction: features::shared_construction(&left),
            features: vec![
                linked_feature(
                    "mount-1",
                    MechanicalFeatureKind::Mount,
                    MechanicalFeatureValue::Mount {
                        mount: mount("mount-1-left", 2.0, 3.0),
                    },
                    MechanicalFeatureState::Local,
                    MechanicalFeatureState::Linked,
                ),
                linked_feature(
                    "opening-1",
                    MechanicalFeatureKind::Opening,
                    MechanicalFeatureValue::Opening {
                        opening: boardstudio_core::model::MechanicalOpening {
                            id: "opening-1".into(),
                            vertices: vec![],
                            z: 1.0,
                            height: 2.0,
                        },
                    },
                    MechanicalFeatureState::Linked,
                    MechanicalFeatureState::Linked,
                ),
                linked_feature(
                    "support-1",
                    MechanicalFeatureKind::GasketSupport,
                    MechanicalFeatureValue::GasketSupport {
                        anchor: anchor("support-1-left", 0.25),
                    },
                    MechanicalFeatureState::Omitted,
                    MechanicalFeatureState::Linked,
                ),
            ],
        };
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            transport: HardwareTransport::None,
            instances: vec![
                PhysicalBoardInstance {
                    mechanical: Some(left),
                    ..instance("left", "board")
                },
                PhysicalBoardInstance {
                    mechanical: Some(right),
                    ..instance("right", "board")
                },
            ],
            boards: vec![],
            shared_construction: None,
            pairs: vec![pair],
        });
        doc
    }

    #[test]
    fn projection_reports_each_linked_feature_and_its_accepted_state() {
        let doc = document();
        let projection = pair_projection(&doc, &[], "left").expect("the half belongs to a pair");
        assert_eq!(projection.pair_id, "pair-1");
        assert_eq!(
            projection.members,
            ["left".to_string(), "right".to_string()]
        );
        let kinds: Vec<_> = projection
            .features
            .iter()
            .map(|row| (row.feature_id.as_str(), row.kind, row.state))
            .collect();
        assert_eq!(
            kinds,
            vec![
                (
                    "mount-1-left",
                    MechanicalFeatureKind::Mount,
                    MechanicalFeatureState::Local
                ),
                (
                    "opening-1-left",
                    MechanicalFeatureKind::Opening,
                    MechanicalFeatureState::Linked
                ),
                (
                    "support-1-left",
                    MechanicalFeatureKind::GasketSupport,
                    MechanicalFeatureState::Omitted
                ),
            ],
            "the states come from the accepted pair store, covering mounts, openings and anchors"
        );
        assert_eq!(projection.features[0].counterpart_instance_id, "right");
        assert_eq!(
            projection.features[0].counterpart_feature_id,
            "mount-1-right"
        );
        assert!(
            projection.features[0].rejoin_fingerprint.is_some(),
            "each row carries the fingerprint of the reviewed rejoin inputs"
        );
        assert_eq!(
            projection.features[0].preview.as_deref(),
            Some("Mount at (-2.0, 3.0)"),
            "a rejoin previews the geometry the source half actually shows"
        );
    }

    #[test]
    fn projection_is_absent_for_an_unpaired_or_unknown_half() {
        let mut doc = document();
        doc.hardware.as_mut().unwrap().pairs.clear();
        assert_eq!(pair_projection(&doc, &[], "left"), None);
        let doc = document();
        assert_eq!(pair_projection(&doc, &[], "missing"), None);
        let mut without_hardware = ProjectDoc::empty("doc", "Fixture");
        without_hardware.boards.push(doc.boards[0].clone());
        assert_eq!(pair_projection(&without_hardware, &[], "left"), None);
    }

    /// Two independent halves: one shared mount id and one half-local id.
    fn independent_halves() -> ProjectDoc {
        let mut doc = ProjectDoc::empty("doc", "Fixture");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let mut left = configuration("board");
        left.mounts = vec![mount("mount-1", 2.0, 3.0), mount("mount-left", 8.0, 3.0)];
        let mut right = configuration("board");
        right.mounts = vec![mount("mount-1", -2.0, 3.0), mount("mount-right", -8.0, 3.0)];
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            transport: HardwareTransport::None,
            instances: vec![
                PhysicalBoardInstance {
                    mechanical: Some(left),
                    flipped: false,
                    ..instance("left", "board")
                },
                PhysicalBoardInstance {
                    mechanical: Some(right),
                    flipped: true,
                    ..instance("right", "board")
                },
            ],
            boards: vec![],
            shared_construction: None,
            pairs: vec![],
        });
        doc
    }

    #[test]
    fn link_options_offer_each_other_half_with_its_map_and_durable_matches() {
        let doc = independent_halves();
        let options = pair_link_options(&doc, &[], "left");
        assert_eq!(options.len(), 1, "the other half is the one candidate");
        let option = &options[0];
        assert_eq!(option.instance_id, "left");
        assert_eq!(option.other_instance_id, "right");
        assert_eq!(option.pair_id, "pair/left");
        assert_eq!(
            option.derived_mapping,
            Some(Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            }),
            "opposite declared transforms imply the physical reflection"
        );
        assert_eq!(
            option.correspondences.len(),
            1,
            "only the shared durable id matches; the half-local mount stays local"
        );
        assert_eq!(option.matched_features, "Mount mount-1");
        assert!(
            option.reviewed_fingerprint.is_some(),
            "the review carries the fingerprint of the inputs it was produced from"
        );
        assert!(pair_link_options(&doc, &[], "missing").is_empty());
    }

    #[test]
    fn link_options_need_an_explicit_map_for_translated_frames_and_are_absent_when_paired() {
        let mut doc = independent_halves();
        doc.hardware.as_mut().unwrap().instances[1].flipped = false;
        let options = pair_link_options(&doc, &[], "left");
        assert_eq!(
            options[0].derived_mapping, None,
            "equal declared transforms need the review to supply the map"
        );
        assert_eq!(
            options[0].correspondences.len(),
            1,
            "the matches are still reviewable without a derived map"
        );

        let mut paired = doc.clone();
        paired.hardware.as_mut().unwrap().pairs = vec![MechanicalMirrorPair {
            id: "pair-1".into(),
            members: ["left".into(), "right".into()],
            mapping: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            construction: features::shared_construction(&configuration("board")),
            features: vec![],
        }];
        assert!(
            pair_link_options(&paired, &[], "left").is_empty(),
            "a paired half is linked through its pair actions instead"
        );
        assert!(pair_link_options(&paired, &[], "right").is_empty());
    }

    fn rejoin(instance_id: &str, feature_id: &str, source: &str) -> MechanicalChange {
        MechanicalChange::RejoinFeatureMember {
            feature_id: feature_id.into(),
            instance_id: instance_id.into(),
            source_instance_id: source.into(),
        }
    }

    #[test]
    fn the_rejoin_fingerprint_tolerates_unrelated_edits_and_revision_changes() {
        let doc = document();
        let change = rejoin("left", "mount-1-left", "right");
        let reviewed = reviewed_fingerprint(&doc, &[], &change).expect("the reviewed inputs exist");

        let mut unrelated = doc.clone();
        unrelated.name = "Renamed".into();
        unrelated.revision += 7;
        unrelated
            .parameters
            .insert("unrelated".into(), json!("keep"));
        unrelated.hardware.as_mut().unwrap().instances[0]
            .mechanical
            .as_mut()
            .unwrap()
            .wall_thickness = 2.5;
        assert_eq!(
            reviewed_fingerprint(&unrelated, &[], &change).unwrap(),
            reviewed,
            "unrelated edits and a revision change do not retire the review"
        );

        // Each relevant input does change it.
        let mut moved = doc.clone();
        moved.hardware.as_mut().unwrap().pairs[0].mapping.rotation = 90.0;
        assert_ne!(
            reviewed_fingerprint(&moved, &[], &change).unwrap(),
            reviewed
        );

        let mut authored = doc.clone();
        authored.hardware.as_mut().unwrap().pairs[0].features[0].value =
            MechanicalFeatureValue::Mount {
                mount: mount("mount-1-left", 9.0, 3.0),
            };
        assert_ne!(
            reviewed_fingerprint(&authored, &[], &change).unwrap(),
            reviewed
        );

        let mut membership = doc.clone();
        membership.hardware.as_mut().unwrap().pairs[0].members[1] = "other".into();
        membership.hardware.as_mut().unwrap().pairs[0]
            .features
            .iter_mut()
            .for_each(|feature| feature.members[1].instance_id = "other".into());
        assert_ne!(
            reviewed_fingerprint(&membership, &[], &change).unwrap(),
            reviewed
        );

        let mut state = doc.clone();
        state.hardware.as_mut().unwrap().pairs[0].features[0].members[0].state =
            MechanicalFeatureState::Linked;
        assert_ne!(
            reviewed_fingerprint(&state, &[], &change).unwrap(),
            reviewed
        );

        assert_eq!(
            reviewed_fingerprint(&doc, &[], &rejoin("left", "mount-1-left", "right")).unwrap(),
            reviewed_fingerprint(&doc, &[], &rejoin("left", "mount-1-left", "left")).unwrap(),
            "the reviewed source choice is the user's payload, not accepted-document state"
        );
    }

    #[test]
    fn a_vanished_reviewed_target_has_no_fingerprint() {
        let doc = document();
        assert!(
            reviewed_fingerprint(&doc, &[], &rejoin("left", "missing", "right")).is_err(),
            "a feature that is no longer linked has no reviewed inputs"
        );
        assert!(
            reviewed_fingerprint(&doc, &[], &rejoin("missing", "mount-1-left", "right")).is_err()
        );
        assert!(reviewed_fingerprint(&doc, &[], &MechanicalChange::Enable).is_err());
    }

    fn correspondences()
    -> Vec<boardstudio_core::mechanical::change::MechanicalFeatureCorrespondence> {
        vec![
            boardstudio_core::mechanical::change::MechanicalFeatureCorrespondence {
                kind: MechanicalFeatureKind::Mount,
                first_feature_id: "mount-1-left".into(),
                second_feature_id: "mount-1-right".into(),
            },
        ]
    }

    fn link(doc: &ProjectDoc, mapping: Pose2) -> MechanicalChange {
        let _ = doc;
        MechanicalChange::LinkMirrorPair {
            pair_id: "pair-new".into(),
            members: ["left".into(), "right".into()],
            mapping,
            construction_source: "left".into(),
            correspondences: correspondences(),
        }
    }

    #[test]
    fn the_link_fingerprint_covers_the_accepted_inputs_and_ignores_the_reviewed_choices() {
        let mut doc = document();
        doc.hardware.as_mut().unwrap().pairs.clear();
        let change = link(
            &doc,
            Pose2 {
                at: Vec2 { x: 30.0, y: -5.0 },
                rotation: 180.0,
            },
        );
        let reviewed = reviewed_fingerprint(&doc, &[], &change).expect("the reviewed inputs exist");

        let mut renamed = doc.clone();
        renamed.name = "Renamed".into();
        renamed.revision += 3;
        renamed.parameters.insert("unrelated".into(), json!("keep"));
        assert_eq!(
            reviewed_fingerprint(&renamed, &[], &change).unwrap(),
            reviewed,
            "unrelated project edits and a revision change do not retire a reviewed link"
        );

        // The map, the construction source and the source choices are the reviewed
        // payload: changing them is not document staleness.
        let other_choices = MechanicalChange::LinkMirrorPair {
            pair_id: "pair-new".into(),
            members: ["left".into(), "right".into()],
            mapping: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            construction_source: "right".into(),
            correspondences: correspondences(),
        };
        assert_eq!(
            reviewed_fingerprint(&doc, &[], &other_choices).unwrap(),
            reviewed,
            "the reviewed map and construction source are the user's own choices"
        );

        // A declared-transform change alters the map a review derives, so it retires.
        let mut reoriented = doc.clone();
        reoriented.hardware.as_mut().unwrap().instances[1].flipped = true;
        assert_ne!(
            reviewed_fingerprint(&reoriented, &[], &change).unwrap(),
            reviewed,
            "changing a half's declared transform retires the review"
        );

        // Moving the geometry a match consolidates retires it too.
        let mut moved = doc.clone();
        moved.hardware.as_mut().unwrap().instances[0]
            .mechanical
            .as_mut()
            .unwrap()
            .mounts = vec![mount("mount-1-left", 8.0, 1.0)];
        assert_ne!(
            reviewed_fingerprint(&moved, &[], &change).unwrap(),
            reviewed,
            "moving a matched feature retires the review"
        );

        let mut already_paired = doc.clone();
        let mut stored = document().hardware.unwrap().pairs;
        stored[0].id = "pair-new".into();
        already_paired.hardware.as_mut().unwrap().pairs = stored;
        assert!(
            reviewed_fingerprint(&already_paired, &[], &change).is_err(),
            "an existing pair id makes the reviewed link ineligible"
        );
    }

    #[test]
    fn a_gasket_anchor_rejoin_previews_the_source_geometry_it_would_take() {
        let doc = document();
        let projection = pair_projection(&doc, &[], "left").expect("the half is paired");
        let support = projection
            .features
            .iter()
            .find(|row| row.kind == MechanicalFeatureKind::GasketSupport)
            .expect("the anchor row exists");
        assert_eq!(support.state, MechanicalFeatureState::Omitted);
        assert_eq!(
            support.preview.as_deref(),
            Some("Gasket support at 0.25 along its region"),
            "an omitted anchor previews the source half's visible anchor"
        );

        // An omitted source half has no geometry to rejoin with, so no preview.
        let right = pair_projection(&doc, &[], "right").expect("the other half is paired too");
        let omitted_source = right
            .features
            .iter()
            .find(|row| row.kind == MechanicalFeatureKind::GasketSupport)
            .expect("the anchor row exists");
        assert_eq!(omitted_source.preview, None);
    }
}
