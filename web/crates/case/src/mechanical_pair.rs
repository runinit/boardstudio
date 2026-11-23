//! The Case view of a physical half's mechanical mirror pair, and the reviewed-input
//! fingerprint its per-feature actions are submitted with.
//!
//! Every row is derived from the accepted pair store, so there is no second mutable
//! relationship store. The fingerprint covers exactly the inputs a reviewed link or
//! rejoin depends on - membership, mapping, the chosen feature identity, its kind and
//! the value the result is derived from, plus the pair's shared construction when a
//! link consolidates it - so unrelated edits and revision changes leave a review valid
//! while a relevant input change retires it.
mod fingerprint;
pub use fingerprint::reviewed_fingerprint;

use boardstudio_application::CoreEngine;
use boardstudio_core_mechanical::mechanical::MechanicalTarget;
use boardstudio_core_mechanical::mechanical::change::{
    MechanicalChange, MechanicalEdit, MechanicalFeatureCorrespondence,
};
use boardstudio_core_mechanical::mechanical::features;
use boardstudio_core_mechanical::mechanical::inputs::effective_inputs;
use boardstudio_model::model::{
    BoardContours, EditCommand, EditOperation, EditPhase, MechanicalConfiguration,
    MechanicalFeature, MechanicalFeatureKind, MechanicalFeatureMember, MechanicalFeatureState,
    MechanicalFeatureValue, MechanicalMirrorPair, PartDefinition, Pose2, ProjectDoc, Vec2,
};
use std::rc::Rc;

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
                own_geometry: rejoin_preview(document, board_contours, pair, feature, instance_id),
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

/// One matched feature of a candidate link, with the geometry each half shows.
///
/// The reviewed source choice is per feature: it selects which half's accepted geometry
/// becomes the shared authored value, independently of the pair's construction source.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalPairMatchedFeature {
    pub kind: MechanicalFeatureKind,
    pub first_feature_id: String,
    pub second_feature_id: String,
    /// The geometry the first (reviewed) half shows today.
    pub first_geometry: Option<String>,
    /// The geometry the other half shows today.
    pub second_geometry: Option<String>,
    /// The source this review starts from; the user may choose the other half per feature.
    pub source_instance_id: String,
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
    /// The same matches with each half's geometry and the reviewed per-feature source.
    pub matched: Vec<MechanicalPairMatchedFeature>,
    /// The fingerprint of the accepted inputs this candidate's review was produced
    /// from. The link submission carries it for the same reason as a rejoin row's.
    pub reviewed_fingerprint: Option<String>,
    /// The complete proposed result on both halves, read from Core's own read path.
    /// `None` when the projection is unavailable; `preview_note` then says why.
    pub preview: Option<MechanicalLinkPreview>,
    /// Why the complete proposed result is unavailable, when it is.
    pub preview_note: Option<String>,
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
    mounting_hole: Option<&PartDefinition>,
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
            // Each match names the geometry both halves show today and starts from the
            // reviewed half as its source; the user may choose the other half per feature.
            let matched = review
                .correspondences
                .iter()
                .map(|correspondence| {
                    let geometry = |half: &str, feature_id: &str| {
                        effective_inputs(
                            document,
                            board_contours,
                            &MechanicalTarget::Instance {
                                instance_id: half.to_owned(),
                            },
                        )
                        .ok()
                        .and_then(|inputs| {
                            configuration_feature(
                                &inputs.configuration,
                                correspondence.kind,
                                feature_id,
                            )
                            .map(|value| describe_value(correspondence.kind, &value))
                        })
                    };
                    MechanicalPairMatchedFeature {
                        kind: correspondence.kind,
                        first_feature_id: correspondence.first_feature_id.clone(),
                        second_feature_id: correspondence.second_feature_id.clone(),
                        first_geometry: geometry(instance_id, &correspondence.first_feature_id),
                        second_geometry: geometry(&other.id, &correspondence.second_feature_id),
                        source_instance_id: instance_id.to_owned(),
                    }
                })
                .collect::<Vec<_>>();
            // The reviewed payload names the source per feature explicitly.
            let reviewed_correspondences = review
                .correspondences
                .iter()
                .map(|correspondence| MechanicalFeatureCorrespondence {
                    source_instance_id: Some(instance_id.to_owned()),
                    ..correspondence.clone()
                })
                .collect::<Vec<_>>();
            // The complete proposed result comes from Core's own read path, read-only, and
            // only for the map the review actually uses. A candidate whose map the user
            // still has to enter is not projected here: showing a result computed from a
            // placeholder map would misstate a translated or rotated frame.
            let (preview, preview_note) = match review.derived_mapping {
                Some(mapping) => {
                    let reviewed = MechanicalChange::LinkMirrorPair {
                        pair_id: pair_id.clone(),
                        members: [instance_id.to_owned(), other.id.clone()],
                        mapping,
                        construction_source: review.construction_source.clone(),
                        correspondences: reviewed_correspondences.clone(),
                    };
                    match link_preview(
                        document,
                        board_contours,
                        &reviewed,
                        mounting_hole,
                    ) {
                        Ok(preview) => (Some(preview), None),
                        Err(reason) => (None, Some(reason)),
                    }
                }
                None => (
                    None,
                    Some(
                        "the reviewed map for this board frame is entered before the result is projected"
                            .to_owned(),
                    ),
                ),
            };
            // The fingerprint covers the accepted inputs the review was produced from;
            // the map and the source choices are the payload it is confirmed with.
            let reviewed_fingerprint = reviewed_fingerprint(
                document,
                board_contours,
                &MechanicalChange::LinkMirrorPair {
                    pair_id: pair_id.clone(),
                    members: [instance_id.to_owned(), other.id.clone()],
                    mapping: review.derived_mapping.unwrap_or(Pose2 {
                        at: Vec2::default(),
                        rotation: 0.0,
                    }),
                    construction_source: review.construction_source.clone(),
                    correspondences: reviewed_correspondences.clone(),
                },
            )
            .ok();
            Some(MechanicalPairLinkOption {
                pair_id,
                instance_id: instance_id.to_owned(),
                other_instance_id: other.id.clone(),
                other_name: other.name.clone(),
                derived_mapping: review.derived_mapping,
                construction_source: review.construction_source,
                matched_features: matched_features(&review.correspondences),
                matched,
                correspondences: reviewed_correspondences,
                reviewed_fingerprint,
                preview,
                preview_note,
            })
        })
        .collect()
}

/// One half's read-only view of what a reviewed link produces.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalLinkMemberPreview {
    pub instance_id: String,
    /// Every visible mechanical feature of that half after the link, described for the
    /// review. These come from Core's effective read of the projected result.
    pub features: Vec<String>,
    /// Findings the projected result carries for that half.
    pub findings: Vec<String>,
}

/// The complete proposed result of a reviewed link, for both halves.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalLinkPreview {
    pub members: Vec<MechanicalLinkMemberPreview>,
}

/// Project what a reviewed link produces on both halves, read-only.
///
/// A disposable engine applies the reviewed typed edit to a clone of the accepted
/// document; the resulting document and scene are then read through Core's
/// effective-input path, so both halves' visible geometry and findings come from Core's
/// own rules. The transient result is never submitted, stored, carried in a payload or
/// given any ownership of accepted state: the confirmation still sends the typed
/// choices, ids and template against the operation-time accepted document.
pub fn link_preview(
    document: &ProjectDoc,
    board_contours: &[BoardContours],
    change: &MechanicalChange,
    mounting_hole: Option<&PartDefinition>,
) -> Result<MechanicalLinkPreview, String> {
    let MechanicalChange::LinkMirrorPair { members, .. } = change else {
        return Err("Only a reviewed link has a two-half preview.".into());
    };
    let mut core = CoreEngine::new();
    core.open(document.clone())?;
    let edit = MechanicalEdit {
        target: MechanicalTarget::Instance {
            instance_id: members[0].clone(),
        },
        change: change.clone(),
        mounting_hole: mounting_hole.map(|template| Box::new(template.clone())),
    };
    let command = EditCommand {
        base_revision: document.revision,
        transaction_id: "pair-review".into(),
        phase: EditPhase::Commit,
        target_ids: members.to_vec(),
        operation: EditOperation::ApplyMechanicalEdit {
            edit: Box::new(edit),
        },
    };
    let projected = core.commit_edit(command)?.document;

    let mut previews = Vec::with_capacity(members.len());
    for instance_id in members {
        // Board outlines are the accepted ones: linking changes mechanical features,
        // not the board frames the effective read is based on.
        let inputs = effective_inputs(
            &projected,
            board_contours,
            &MechanicalTarget::Instance {
                instance_id: instance_id.clone(),
            },
        )?;
        let configuration = inputs.configuration;
        let mut features = Vec::new();
        for mount in &configuration.mounts {
            features.push(describe_value(
                MechanicalFeatureKind::Mount,
                &MechanicalFeatureValue::Mount {
                    mount: mount.clone(),
                },
            ));
        }
        for mount in configuration.closure_mounts.as_deref().unwrap_or_default() {
            features.push(describe_value(
                MechanicalFeatureKind::ClosureMount,
                &MechanicalFeatureValue::Mount {
                    mount: mount.clone(),
                },
            ));
        }
        for opening in configuration.openings.as_deref().unwrap_or_default() {
            features.push(describe_value(
                MechanicalFeatureKind::Opening,
                &MechanicalFeatureValue::Opening {
                    opening: opening.clone(),
                },
            ));
        }
        // Gasket supports are stated from Core's resolved geometry - resolved on the
        // authoring half's contours and reflected here - never from the authored fraction,
        // which is not where the support lands. A support that is not linked yet has no
        // resolved placement, so it is named without inventing one.
        let resolved: std::collections::BTreeSet<&str> = inputs
            .gasket_supports
            .iter()
            .map(|support| support.id.as_str())
            .collect();
        for support in &inputs.gasket_supports {
            let mut row = format!(
                "Gasket support at ({:?}, {:?}) · {:?} × {:?} mm",
                support.at.x, support.at.y, support.length, support.width
            );
            if let Some(error) = support.fit_error.as_deref() {
                row.push_str(&format!(" · does not fit: {error}"));
            }
            features.push(row);
        }
        for anchor in configuration
            .gasket_layout
            .as_ref()
            .map(|layout| layout.supports.as_slice())
            .unwrap_or_default()
        {
            if resolved.contains(anchor.id.as_str()) {
                continue;
            }
            features.push(format!(
                "Gasket support on region {:?} (its placement resolves when the feature is linked)",
                anchor.region_id
            ));
        }
        // Core's accepted mechanical findings for this half, recomputed against the
        // projected state; the panel does not invent or filter findings itself.
        let findings = inputs
            .findings
            .iter()
            .map(|finding| format!("{:?}: {}", finding.severity, finding.message))
            .collect();
        previews.push(MechanicalLinkMemberPreview {
            instance_id: instance_id.clone(),
            features,
            findings,
        });
    }
    Ok(MechanicalLinkPreview { members: previews })
}

/// A read-only projection of a reviewed link's complete result, owned by the mount.
///
/// The mount captures the accepted document, its accepted contours and the normalized
/// mounting-hole template, then applies the reviewed typed edit through a disposable
/// Core engine and reads the result back through Core's effective-input path. The
/// transient document never leaves the closure: it is never submitted to Session, never
/// carried in a payload and never given ownership of accepted state. Calling the port
/// is pure with respect to the accepted document.
/// The projection function a mount supplies to the review.
pub type LinkProjection = dyn Fn(&MechanicalChange) -> Result<MechanicalLinkPreview, String>;

#[derive(Clone)]
pub struct MechanicalPairProjectionPort(pub Rc<LinkProjection>);

impl PartialEq for MechanicalPairProjectionPort {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The typed link a review actually shows, from its own choices.
///
/// This is the one place the reviewed choices become a payload: the map (derived or
/// entered), which half supplies construction, and which half supplies each matched
/// feature. `None` means the review has no map yet, so there is nothing to project or
/// submit. Nothing here computes geometry; it only names what the user chose.
pub fn reviewed_link_change(
    option: &MechanicalPairLinkOption,
    mapping: Option<Pose2>,
    construction_from_other: bool,
    sources: &std::collections::BTreeMap<String, String>,
) -> Option<MechanicalChange> {
    let mapping = mapping.or(option.derived_mapping)?;
    let construction_source = if construction_from_other {
        option.other_instance_id.clone()
    } else {
        option.instance_id.clone()
    };
    let correspondences = option
        .matched
        .iter()
        .map(|matched| MechanicalFeatureCorrespondence {
            kind: matched.kind,
            first_feature_id: matched.first_feature_id.clone(),
            second_feature_id: matched.second_feature_id.clone(),
            source_instance_id: Some(
                sources
                    .get(&matched.first_feature_id)
                    .cloned()
                    .unwrap_or_else(|| matched.source_instance_id.clone()),
            ),
        })
        .collect();
    Some(MechanicalChange::LinkMirrorPair {
        pair_id: option.pair_id.clone(),
        members: [option.instance_id.clone(), option.other_instance_id.clone()],
        mapping,
        construction_source,
        correspondences,
    })
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

#[cfg(test)]
mod tests;
