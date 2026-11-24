use super::*;

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
    let destination = features::feature_member(feature, instance_id).ok_or_else(|| {
        format!("Physical instance {instance_id:?} has no linked feature {feature_id:?}.")
    })?;
    let counterpart = pair
        .members
        .iter()
        .find(|member| member.as_str() != instance_id)
        .ok_or("The mechanical pair has no other member.")?;
    let counterpart_record = features::feature_member(feature, counterpart).ok_or_else(|| {
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
    correspondences: &[boardstudio_core_mechanical::mechanical::change::MechanicalFeatureCorrespondence],
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
