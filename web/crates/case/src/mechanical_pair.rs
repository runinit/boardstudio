//! The Case view of a physical half's mechanical mirror pair. Every row is derived from the
//! accepted pair store, so there is no second mutable relationship store.

use boardstudio_core_mechanical::mechanical::MechanicalTarget;
use boardstudio_core_mechanical::mechanical::inputs::effective_inputs;
use boardstudio_model::model::{
    BoardContours, MechanicalConfiguration, MechanicalFeature, MechanicalFeatureKind,
    MechanicalFeatureMember, MechanicalFeatureState, MechanicalFeatureValue, MechanicalMirrorPair,
    ProjectDoc,
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
    /// The geometry this half itself shows, described for review. A local exception can
    /// be rejoined from its own current geometry, keeping this half's placement and
    /// re-deriving the other; an omitted half has none.
    pub own_geometry: Option<String>,
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
                own_geometry: rejoin_preview(document, board_contours, pair, feature, instance_id),
            })
        })
        .collect();
    Some(MechanicalPairProjection {
        pair_id: pair.id.clone(),
        members: pair.members.clone(),
        features,
    })
}

/// What a rejoin would take from `source_instance_id`, read through Core's effective-input path.
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
    // A gasket support's reviewed geometry is its resolved placement, not the authored
    // fraction it is anchored at; a support that is not resolved yet still names its
    // feature rather than inventing a placement.
    if feature.kind == MechanicalFeatureKind::GasketSupport
        && let Some(support) = inputs
            .gasket_supports
            .iter()
            .find(|support| support.id == source.feature_id)
    {
        let mut description = format!(
            "{:?} at ({:?}, {:?}) · {:?} × {:?} mm",
            feature_label(feature.kind),
            support.at.x,
            support.at.y,
            support.length,
            support.width
        );
        if let Some(error) = support.fit_error.as_deref() {
            description.push_str(&format!(" · does not fit: {error}"));
        }
        return Some(description);
    }
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
