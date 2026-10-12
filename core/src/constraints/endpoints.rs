use crate::constraints::relation::ConstraintRelation;
use crate::mechanical::pair;
use crate::model::{
    MechanicalMirrorPair, MirroredCompanionMode, MirroredCorrespondence, MirroredExceptionState,
    MirroredHalves, MirroredItemKind, Pose2, ProjectDoc, Vec2,
};
use serde::{Deserialize, Serialize};

/// The authored, visible identity of a framed relation endpoint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum ConstraintEndpoint {
    Part {
        #[serde(rename = "partId")]
        part_id: String,
    },
    LinkedMirror {
        #[serde(rename = "pairId")]
        pair_id: String,
        #[serde(rename = "sourcePartId")]
        source_part_id: String,
        #[serde(rename = "mirrorPartId")]
        mirror_part_id: String,
    },
}

/// Private wire-shape candidate; the existing global constraint ID stays attached to the record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum FramedConstraint {
    Framed {
        id: String,
        source: ConstraintEndpoint,
        target: ConstraintEndpoint,
        relation: ConstraintRelation,
    },
}

#[cfg_attr(not(test), expect(dead_code))] // Only tests read these accessors; the solver binds through resolve_endpoint.
impl ConstraintEndpoint {
    pub(super) fn visible_part_id(&self) -> &str {
        match self {
            Self::Part { part_id } => part_id,
            Self::LinkedMirror { mirror_part_id, .. } => mirror_part_id,
        }
    }

    pub(super) fn authoritative_part_id(&self) -> &str {
        match self {
            Self::Part { part_id } => part_id,
            Self::LinkedMirror { source_part_id, .. } => source_part_id,
        }
    }
}

/// How a mirrored companion sits against its mirrored key (ADR-0010).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum CompanionRule {
    /// Reflect the companion's pose like the key's.
    Literal,
    /// Keep the source's world heading; the position is reflected.
    Stabiliser,
    /// Keep the source's key-local position and heading.
    Duplicate,
    /// Reflect, then add `(u, v)` in the mirrored key's frame and `w` to the heading.
    Tweaked { u: f64, v: f64, w: f64 },
}

/// A companion rule written once as a map from the authoritative pose to the visible one, so
/// `project` and `unproject` cannot drift apart.
///
/// Position: with no `turn`, `forward(mapping, p) + shift`; with a `turn`,
/// `origin + R(turn) (p - pivot)`. Heading: `heading_sign * c + heading_base`, the sign being
/// +1 or -1.
struct CompanionMap {
    turn: Option<f64>,
    pivot: Vec2,
    origin: Vec2,
    shift: Vec2,
    heading_sign: f64,
    heading_base: f64,
}

impl CompanionRule {
    /// The rule's map for a pair `mapping` and the resolved key pose it is measured against.
    fn frame(self, mapping: Pose2, key: Pose2) -> CompanionMap {
        let reflected = |shift: Vec2, heading_sign: f64, heading_base: f64| CompanionMap {
            turn: None,
            pivot: Vec2::default(),
            origin: Vec2::default(),
            shift,
            heading_sign,
            heading_base,
        };
        match self {
            Self::Literal => reflected(Vec2::default(), -1.0, mapping.rotation),
            Self::Stabiliser => reflected(Vec2::default(), 1.0, 0.0),
            Self::Tweaked { u, v, w } => reflected(
                turned(Vec2 { x: u, y: v }, mapping.rotation - key.rotation),
                -1.0,
                mapping.rotation + w,
            ),
            Self::Duplicate => {
                let turn = mapping.rotation - 2.0 * key.rotation;
                CompanionMap {
                    turn: Some(turn),
                    pivot: key.at,
                    origin: pair::forward(mapping, key.at),
                    shift: Vec2::default(),
                    heading_sign: 1.0,
                    heading_base: turn,
                }
            }
        }
    }
}

/// A live endpoint frame. Linked mappings are supplied by the current pair owner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum EndpointFrame {
    Identity,
    /// An independent mirrored part: the full heading is turned half a turn.
    LinkedMirror {
        mapping: Pose2,
    },
    /// A mirrored matrix key.
    LinkedKey {
        mapping: Pose2,
    },
    /// A mirrored companion. `key` is the source key's resolved authoritative pose, which the
    /// solver supplies once the key's owner is resolved; it is never frozen in the document.
    /// `None` is a frame not yet bound: a rule that reads the key refuses to run against it.
    LinkedCompanion {
        mapping: Pose2,
        key: Option<Pose2>,
        rule: CompanionRule,
    },
}

impl EndpointFrame {
    /// A companion frame with the resolved pose of the key it is measured against; other
    /// frames do not depend on a key and are returned as they are.
    pub(super) fn with_key(self, resolved: Option<Pose2>) -> Self {
        match (self, resolved) {
            (Self::LinkedCompanion { mapping, rule, .. }, Some(key)) => Self::LinkedCompanion {
                mapping,
                key: Some(key),
                rule,
            },
            (frame, _) => frame,
        }
    }

    pub(super) fn project(self, authoritative: Pose2) -> Result<Pose2, String> {
        validate_pose(authoritative)?;
        let projected = match self {
            Self::Identity => authoritative,
            Self::LinkedMirror { mapping } => {
                validate_mapping(mapping)?;
                Pose2 {
                    at: pair::forward(mapping, authoritative.at),
                    rotation: (mapping.rotation + 180.0 - authoritative.rotation).rem_euclid(360.0),
                }
            }
            Self::LinkedKey { mapping } => {
                validate_mapping(mapping)?;
                Pose2 {
                    at: pair::forward(mapping, authoritative.at),
                    rotation: mapping.rotation - authoritative.rotation,
                }
            }
            Self::LinkedCompanion { mapping, key, rule } => {
                validate_mapping(mapping)?;
                let map = rule.frame(mapping, bound_key(key, rule)?);
                let at = match map.turn {
                    None => {
                        let reflected = pair::forward(mapping, authoritative.at);
                        Vec2 {
                            x: reflected.x + map.shift.x,
                            y: reflected.y + map.shift.y,
                        }
                    }
                    Some(turn) => {
                        let local = turned(
                            Vec2 {
                                x: authoritative.at.x - map.pivot.x,
                                y: authoritative.at.y - map.pivot.y,
                            },
                            turn,
                        );
                        Vec2 {
                            x: map.origin.x + local.x,
                            y: map.origin.y + local.y,
                        }
                    }
                };
                Pose2 {
                    at,
                    rotation: map.heading_sign * authoritative.rotation + map.heading_base,
                }
            }
        };
        validate_pose(projected)?;
        Ok(projected)
    }

    pub(super) fn unproject(
        self,
        visible: Pose2,
        previous_authoritative: Pose2,
    ) -> Result<Pose2, String> {
        validate_pose(visible)?;
        validate_pose(previous_authoritative)?;
        let authoritative = match self {
            Self::Identity => visible,
            Self::LinkedMirror { mapping } => {
                validate_mapping(mapping)?;
                let base_rotation = mapping.rotation + 180.0 - visible.rotation;
                Pose2 {
                    at: pair::inverse(mapping, visible.at),
                    rotation: nearest_equivalent(base_rotation, previous_authoritative.rotation),
                }
            }
            Self::LinkedKey { mapping } => {
                validate_mapping(mapping)?;
                Pose2 {
                    at: pair::inverse(mapping, visible.at),
                    rotation: nearest_equivalent(
                        mapping.rotation - visible.rotation,
                        previous_authoritative.rotation,
                    ),
                }
            }
            Self::LinkedCompanion { mapping, key, rule } => {
                validate_mapping(mapping)?;
                let map = rule.frame(mapping, bound_key(key, rule)?);
                let at = match map.turn {
                    None => pair::inverse(
                        mapping,
                        Vec2 {
                            x: visible.at.x - map.shift.x,
                            y: visible.at.y - map.shift.y,
                        },
                    ),
                    Some(turn) => {
                        let local = turned(
                            Vec2 {
                                x: visible.at.x - map.origin.x,
                                y: visible.at.y - map.origin.y,
                            },
                            -turn,
                        );
                        Vec2 {
                            x: map.pivot.x + local.x,
                            y: map.pivot.y + local.y,
                        }
                    }
                };
                // The heading sign is +1 or -1, so it is its own inverse.
                let rotation = (visible.rotation - map.heading_base) * map.heading_sign;
                Pose2 {
                    at,
                    rotation: nearest_equivalent(rotation, previous_authoritative.rotation),
                }
            }
        };
        validate_pose(authoritative)?;
        Ok(authoritative)
    }
}

/// The key a companion rule reads. The literal and stabiliser rules read none, so an unbound
/// frame may stand for them; the others refuse rather than compute against an invented origin.
fn bound_key(key: Option<Pose2>, rule: CompanionRule) -> Result<Pose2, String> {
    match (key, rule) {
        (Some(key), _) => {
            validate_pose(key)?;
            Ok(key)
        }
        (None, CompanionRule::Literal | CompanionRule::Stabiliser) => Ok(Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        }),
        (None, _) => Err("A companion endpoint was used before its key was resolved".into()),
    }
}

fn turned(vector: Vec2, degrees: f64) -> Vec2 {
    crate::matrix::splay::rotate(vector, degrees, Vec2::default())
}

fn nearest_equivalent(angle: f64, reference: f64) -> f64 {
    // f64::round makes exact half-turn ties deterministic, away from zero.
    angle + 360.0 * ((reference - angle) / 360.0).round()
}

fn validate_mapping(mapping: Pose2) -> Result<(), String> {
    validate_pose(mapping)
}

fn validate_pose(pose: Pose2) -> Result<(), String> {
    if pose.at.x.is_finite() && pose.at.y.is_finite() && pose.rotation.is_finite() {
        Ok(())
    } else {
        Err("Constraint endpoint frame contains a non-finite mapping or pose".into())
    }
}

/// Which end of a relation an endpoint is, for located messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Source,
    Target,
}

impl Side {
    fn name(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Target => "target",
        }
    }
}

/// An endpoint bound to the live document: the part the author sees, the part that owns the
/// geometry, and the frame between them.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ResolvedEndpoint {
    pub(super) visible_id: String,
    pub(super) authoritative_id: String,
    /// For a companion frame, `key` is a placeholder until the solver supplies `owner`'s pose.
    pub(super) frame: EndpointFrame,
    /// The authoritative key a companion endpoint is projected against. The solver resolves it
    /// first, so the projection depends on the key's live pose and a cycle through it is refused.
    pub(super) owner: Option<String>,
}

fn state_name(state: MirroredExceptionState) -> &'static str {
    match state {
        MirroredExceptionState::Local => "local",
        MirroredExceptionState::Omitted => "omitted",
    }
}

/// The map from the pair's construction source to its other member: the stored map when the
/// source is the first member, its exact inverse when the source is the second.
fn source_mapping(pair: &MechanicalMirrorPair) -> Result<Pose2, String> {
    match pair.construction_source.as_deref() {
        Some(source) if source == pair.members[0] => Ok(pair.mapping),
        Some(source) if source == pair.members[1] => {
            Ok(crate::mechanical::features::inverse_pose(pair.mapping))
        }
        _ => Err(format!(
            "Pair {} has no construction source to orient its mirror map",
            pair.id
        )),
    }
}

/// What a linked source part is in the matrix domain decides how its mirror is projected: a
/// key, a companion on a key (under the item's effective placement rule), or an independent
/// part. Returns the frame and, for a companion, the key it is measured against. A key or
/// companion of a disabled or deleted cell, or a companion whose key part is absent, is one
/// the mirror never produces: it is refused here rather than projected as an independent part.
fn linked_frame(
    document: &ProjectDoc,
    halves: &MirroredHalves,
    correspondence: &MirroredCorrespondence,
    source_part_id: &str,
    mapping: Pose2,
) -> Result<(EndpointFrame, Option<String>), String> {
    // The rule choice below mirrors `mirrored_half::derive::companion_assembly`, which is the
    // authority for companion placement: keep the two in step.
    for matrix in &document.matrices {
        for cell in &matrix.cells {
            let key_id = crate::matrix::member_id(&matrix.id, cell.row, cell.column);
            let assembly = cell.assemblies.iter().find(|assembly| {
                crate::matrix::companion_id(&key_id, &assembly.id) == source_part_id
            });
            if (key_id == source_part_id || assembly.is_some()) && (!cell.enabled || cell.deleted) {
                return Err(format!(
                    "{source_part_id} belongs to a disabled or deleted cell of matrix {} and is not mirrored",
                    matrix.id
                ));
            }
            let Some(assembly) = assembly else {
                continue;
            };
            if !document.parts.iter().any(|part| part.id == key_id) {
                return Err(format!(
                    "the key {key_id} of companion {source_part_id} is missing"
                ));
            }
            let mode = correspondence
                .companion_mode
                .unwrap_or(halves.companion_mode);
            let rule = if crate::mirrored_half::SYMMETRIC_DEFINITIONS
                .contains(&assembly.definition_id.as_str())
            {
                CompanionRule::Stabiliser
            } else {
                match (mode, correspondence.companion_tweak) {
                    (MirroredCompanionMode::DuplicatePosition, _) => CompanionRule::Duplicate,
                    (MirroredCompanionMode::LiteralMirrorWithTweak, Some(tweak)) => {
                        CompanionRule::Tweaked {
                            u: tweak.x,
                            v: tweak.y,
                            w: tweak.rotation,
                        }
                    }
                    _ => CompanionRule::Literal,
                }
            };
            return Ok((
                EndpointFrame::LinkedCompanion {
                    mapping,
                    key: None,
                    rule,
                },
                Some(key_id),
            ));
        }
    }
    // A matrix lists its companions among its members, so keys are what is left.
    if document
        .matrices
        .iter()
        .any(|matrix| matrix.part_ids.iter().any(|id| id == source_part_id))
    {
        return Ok((EndpointFrame::LinkedKey { mapping }, None));
    }
    Ok((EndpointFrame::LinkedMirror { mapping }, None))
}

/// The pair and correspondence that derive `part_id` while linked: a part correspondence names
/// it as the mirror and no Local or Omitted overlay sits on it. Such a part is not authored.
fn derived_from<'a>(
    document: &'a ProjectDoc,
    part_id: &str,
) -> Option<(&'a MechanicalMirrorPair, &'a MirroredCorrespondence)> {
    let pairs = document
        .hardware
        .as_ref()
        .map(|hardware| hardware.pairs.as_slice())
        .unwrap_or_default();
    pairs.iter().find_map(|pair| {
        let halves = pair.halves.as_ref()?;
        let correspondence = halves.correspondences.iter().find(|correspondence| {
            correspondence.kind == MirroredItemKind::Part
                && correspondence.mirror_id == part_id
                && halves.overlay_of(correspondence).is_none()
        })?;
        Some((pair, correspondence))
    })
}

/// The part that owns `part_id`'s geometry: its source while a live part correspondence derives
/// it, otherwise the part itself. A Local or Omitted mirror is not derived, so it owns itself.
pub(super) fn authoritative_part<'a>(document: &'a ProjectDoc, part_id: &'a str) -> &'a str {
    derived_from(document, part_id).map_or(part_id, |(_, correspondence)| {
        correspondence.source_id.as_str()
    })
}

/// Binds one authored endpoint to the document as it stands. Nothing is cached: a later
/// pair map, correspondence or exception change is read again on the next resolution.
pub(super) fn resolve_endpoint(
    document: &ProjectDoc,
    constraint_id: &str,
    side: Side,
    endpoint: &ConstraintEndpoint,
) -> Result<ResolvedEndpoint, String> {
    let located = |what: String| format!("Constraint {constraint_id} {}: {what}", side.name());
    let has_part = |id: &str| document.parts.iter().any(|part| part.id == id);
    let pairs = document
        .hardware
        .as_ref()
        .map(|hardware| hardware.pairs.as_slice())
        .unwrap_or_default();
    match endpoint {
        ConstraintEndpoint::Part { part_id } => {
            if !has_part(part_id) {
                return Err(located(format!("references a missing part {part_id}")));
            }
            // A part that a live correspondence derives is not authored: it is addressed
            // through its source so the relation survives the mirror being regenerated.
            if let Some((pair, _)) = derived_from(document, part_id) {
                return Err(located(format!(
                    "part {part_id} is a linked mirror in pair {}; address it with a linked-mirror endpoint",
                    pair.id
                )));
            }
            Ok(ResolvedEndpoint {
                visible_id: part_id.clone(),
                authoritative_id: part_id.clone(),
                frame: EndpointFrame::Identity,
                owner: None,
            })
        }
        ConstraintEndpoint::LinkedMirror {
            pair_id,
            source_part_id,
            mirror_part_id,
        } => {
            let pair = pairs
                .iter()
                .find(|pair| pair.id == *pair_id)
                .ok_or_else(|| located(format!("names unknown pair {pair_id}")))?;
            let halves = pair
                .halves
                .as_ref()
                .ok_or_else(|| located(format!("pair {pair_id} has no mirrored halves")))?;
            let correspondence = halves
                .correspondences
                .iter()
                .find(|correspondence| {
                    correspondence.kind == MirroredItemKind::Part
                        && correspondence.source_id == *source_part_id
                        && correspondence.mirror_id == *mirror_part_id
                })
                .ok_or_else(|| {
                    located(format!(
                        "{mirror_part_id} does not match a current part correspondence of {source_part_id} in pair {pair_id}"
                    ))
                })?;
            if let Some(exception) = halves.overlay_of(correspondence) {
                return Err(located(format!(
                    "mirror {mirror_part_id} is {} and is not a linked endpoint",
                    state_name(exception.state)
                )));
            }
            for id in [source_part_id, mirror_part_id] {
                if !has_part(id) {
                    return Err(located(format!("references a missing part {id}")));
                }
            }
            let mapping = source_mapping(pair)?;
            let (frame, owner) =
                linked_frame(document, halves, correspondence, source_part_id, mapping)
                    .map_err(located)?;
            Ok(ResolvedEndpoint {
                visible_id: mirror_part_id.clone(),
                authoritative_id: source_part_id.clone(),
                frame,
                owner,
            })
        }
    }
}

/// Evaluate source → target in their authored endpoint frames, then return target authority.
pub(super) fn solve_target(
    source_authority: Pose2,
    source_frame: EndpointFrame,
    target_frame: EndpointFrame,
    relation: &ConstraintRelation,
    previous_target_authority: Pose2,
) -> Result<Pose2, String> {
    let source_view = source_frame.project(source_authority)?;
    let target_view = relation.apply(source_view)?;
    target_frame.unproject(target_view, previous_target_authority)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::constraints::relation::ConstraintRelation;
    use crate::model::{Pose2, Vec2};
    use serde_json::json;

    const TOLERANCE: f64 = 1e-9;

    #[test]
    fn linked_mirror_address_keeps_visible_identity_separate_from_authority() {
        let endpoint = ConstraintEndpoint::LinkedMirror {
            pair_id: "pair/A".into(),
            source_part_id: "alpha/T".into(),
            mirror_part_id: "beta/T".into(),
        };

        assert_eq!(endpoint.visible_part_id(), "beta/T");
        assert_eq!(endpoint.authoritative_part_id(), "alpha/T");
        assert_eq!(
            serde_json::to_value(endpoint).unwrap(),
            json!({
                "kind": "linked-mirror",
                "pairId": "pair/A",
                "sourcePartId": "alpha/T",
                "mirrorPartId": "beta/T"
            })
        );
    }

    #[test]
    fn part_endpoint_uses_one_identity_for_visible_and_authoritative_address() {
        let endpoint = ConstraintEndpoint::Part {
            part_id: "local/P".into(),
        };

        assert_eq!(endpoint.visible_part_id(), "local/P");
        assert_eq!(endpoint.authoritative_part_id(), "local/P");
        assert_eq!(
            serde_json::to_value(endpoint).unwrap(),
            json!({"kind": "part", "partId": "local/P"})
        );
    }

    #[test]
    fn framed_record_retains_global_id_and_source_to_target_direction() {
        let framed = FramedConstraint::Framed {
            id: "constraint/mixed-7".into(),
            source: ConstraintEndpoint::Part {
                part_id: "local/P".into(),
            },
            target: ConstraintEndpoint::LinkedMirror {
                pair_id: "pair/A".into(),
                source_part_id: "alpha/T".into(),
                mirror_part_id: "beta/T".into(),
            },
            relation: ConstraintRelation::Offset {
                offset: crate::model::Vec2 { x: 5.0, y: -2.0 },
                rotation: 15.0,
            },
        };

        assert_eq!(
            serde_json::to_value(framed.clone()).unwrap(),
            json!({
                "kind": "framed",
                "id": "constraint/mixed-7",
                "source": {"kind": "part", "partId": "local/P"},
                "target": {
                    "kind": "linked-mirror",
                    "pairId": "pair/A",
                    "sourcePartId": "alpha/T",
                    "mirrorPartId": "beta/T"
                },
                "relation": {
                    "kind": "offset",
                    "offset": {"x": 5.0, "y": -2.0},
                    "rotation": 15.0
                }
            })
        );
        let round_trip: FramedConstraint =
            serde_json::from_value(serde_json::to_value(framed.clone()).unwrap()).unwrap();
        assert_eq!(round_trip, framed);
    }

    #[test]
    fn identity_source_and_live_linked_target_solve_in_the_authored_direction() {
        let mapping = Pose2 {
            at: Vec2 { x: 20.0, y: -7.0 },
            rotation: 36.86989764584402,
        };
        let source = Pose2 {
            at: Vec2 { x: 3.0, y: 4.0 },
            rotation: 40.0,
        };
        let relation = ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        };

        let target_authority = solve_target(
            source,
            EndpointFrame::Identity,
            EndpointFrame::LinkedMirror { mapping },
            &relation,
            Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
        )
        .unwrap();

        assert!((target_authority.at.x - 4.2).abs() < TOLERANCE);
        assert!((target_authority.at.y - 14.4).abs() < TOLERANCE);
        assert!((target_authority.rotation - 161.86989764584402).abs() < TOLERANCE);
        let visible_target = EndpointFrame::LinkedMirror { mapping }
            .project(target_authority)
            .unwrap();
        assert!((visible_target.at.x - 8.0).abs() < TOLERANCE);
        assert!((visible_target.at.y - 2.0).abs() < TOLERANCE);
        assert!((visible_target.rotation - 55.0).abs() < TOLERANCE);
    }

    #[test]
    fn live_linked_source_drives_an_identity_local_target() {
        let mapping = Pose2 {
            at: Vec2 { x: 20.0, y: -7.0 },
            rotation: 36.86989764584402,
        };
        let source_authority = Pose2 {
            at: Vec2 { x: 2.0, y: 5.0 },
            rotation: 25.0,
        };
        let relation = ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        };

        let target = solve_target(
            source_authority,
            EndpointFrame::LinkedMirror { mapping },
            EndpointFrame::Identity,
            &relation,
            Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
        )
        .unwrap();

        assert!((target.at.x - 20.4).abs() < TOLERANCE);
        assert!((target.at.y + 6.2).abs() < TOLERANCE);
        assert!((target.rotation - 206.86989764584402).abs() < TOLERANCE);
    }

    #[test]
    fn inverse_linked_heading_chooses_the_equivalent_nearest_previous_heading() {
        let frame = EndpointFrame::LinkedMirror {
            mapping: Pose2 {
                at: Vec2 { x: 20.0, y: -7.0 },
                rotation: 36.86989764584402,
            },
        };
        let viewed = Pose2 {
            at: Vec2 { x: 8.0, y: 2.0 },
            rotation: 55.0,
        };

        let unwrapped = frame
            .unproject(
                viewed,
                Pose2 {
                    at: Vec2 { x: 4.2, y: 14.4 },
                    rotation: 521.869897645844,
                },
            )
            .unwrap();

        assert!((unwrapped.rotation - 521.869897645844).abs() < TOLERANCE);
    }

    #[test]
    fn frame_projection_rejects_non_finite_pair_or_pose_values() {
        let frame = EndpointFrame::LinkedMirror {
            mapping: Pose2 {
                at: Vec2 {
                    x: f64::NAN,
                    y: 0.0,
                },
                rotation: 0.0,
            },
        };
        let pose = Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        };

        assert!(frame.project(pose).is_err());
        assert!(
            EndpointFrame::Identity
                .project(Pose2 {
                    at: Vec2 {
                        x: 0.0,
                        y: f64::INFINITY
                    },
                    rotation: 0.0,
                })
                .is_err()
        );
    }

    pub(in crate::constraints) fn pair_map() -> Pose2 {
        Pose2 {
            at: Vec2 { x: 20.0, y: -7.0 },
            rotation: 36.86989764584402,
        }
    }

    pub(in crate::constraints) fn assert_close(actual: Pose2, x: f64, y: f64, rotation: f64) {
        assert!(
            (actual.at.x - x).abs() < TOLERANCE
                && (actual.at.y - y).abs() < TOLERANCE
                && (actual.rotation - rotation).abs() < TOLERANCE,
            "{actual:?} is not ({x}, {y}) heading {rotation}"
        );
    }

    #[test]
    fn a_mirrored_key_turns_its_heading_without_the_independent_half_turn() {
        let frame = EndpointFrame::LinkedKey {
            mapping: pair_map(),
        };
        let source = Pose2 {
            at: Vec2 { x: 2.0, y: 5.0 },
            rotation: 25.0,
        };
        let visible = frame.project(source).unwrap();
        assert_close(visible, 15.4, -4.2, 11.86989764584402);
        // The independent rule differs by exactly half a turn.
        let independent = EndpointFrame::LinkedMirror {
            mapping: pair_map(),
        }
        .project(source)
        .unwrap();
        assert_close(independent, 15.4, -4.2, 191.86989764584402);
        assert_close(frame.unproject(visible, source).unwrap(), 2.0, 5.0, 25.0);
    }

    #[test]
    fn companion_rules_project_and_invert_to_the_design_oracles() {
        let companion = Pose2 {
            at: Vec2 { x: 4.0, y: 8.0 },
            rotation: 30.0,
        };
        let key_at = |rotation: f64| Pose2 {
            at: Vec2 { x: 2.0, y: 5.0 },
            rotation,
        };
        let cases = [
            (CompanionRule::Literal, 0.0, 12.0, -3.0, 6.86989764584402),
            (CompanionRule::Stabiliser, 0.0, 12.0, -3.0, 30.0),
            (CompanionRule::Stabiliser, 90.0, 12.0, -3.0, 30.0),
            (CompanionRule::Duplicate, 0.0, 15.2, -0.6, 66.86989764584402),
            (
                CompanionRule::Duplicate,
                90.0,
                15.6,
                -7.8,
                -113.13010235415598,
            ),
            (
                CompanionRule::Tweaked {
                    u: 1.0,
                    v: -2.0,
                    w: 7.0,
                },
                0.0,
                14.0,
                -4.0,
                13.86989764584402,
            ),
            (
                CompanionRule::Tweaked {
                    u: 1.0,
                    v: -2.0,
                    w: 7.0,
                },
                90.0,
                11.0,
                -5.0,
                13.86989764584402,
            ),
        ];
        for (rule, key_heading, x, y, heading) in cases {
            let frame = EndpointFrame::LinkedCompanion {
                mapping: pair_map(),
                key: Some(key_at(key_heading)),
                rule,
            };
            let visible = frame.project(companion).unwrap();
            assert_close(visible, x, y, heading);
            assert_close(frame.unproject(visible, companion).unwrap(), 4.0, 8.0, 30.0);
        }
    }

    /// A rule that reads the key must not run against an origin it was never given.
    #[test]
    fn a_companion_rule_that_needs_its_key_refuses_to_run_without_one() {
        let pose = Pose2 {
            at: Vec2 { x: 4.0, y: 8.0 },
            rotation: 30.0,
        };
        for rule in [
            CompanionRule::Duplicate,
            CompanionRule::Tweaked {
                u: 1.0,
                v: -2.0,
                w: 7.0,
            },
        ] {
            let unbound = EndpointFrame::LinkedCompanion {
                mapping: pair_map(),
                key: None,
                rule,
            };
            let projected = unbound.project(pose).unwrap_err();
            assert!(projected.contains("key"), "{projected}");
            assert!(unbound.unproject(pose, pose).is_err());
        }
        // The literal and stabiliser rules do not read the key, so they stand without one.
        for rule in [CompanionRule::Literal, CompanionRule::Stabiliser] {
            let unbound = EndpointFrame::LinkedCompanion {
                mapping: pair_map(),
                key: None,
                rule,
            };
            assert!(unbound.project(pose).is_ok());
        }
    }
}
