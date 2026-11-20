use crate::mechanical::pair;
use crate::model::{
    ConstraintRelation, MechanicalMirrorPair, MirroredCompanionMode, MirroredCorrespondence,
    MirroredExceptionState, MirroredHalves, MirroredItemKind, PartKind, Pose2, ProjectDoc, Vec2,
};
use serde::{Deserialize, Serialize};

pub use crate::model::ConstraintEndpoint;

/// Private wire-shape candidate; the existing global constraint ID stays attached to the record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FramedConstraint {
    Framed {
        id: String,
        source: ConstraintEndpoint,
        target: ConstraintEndpoint,
        relation: ConstraintRelation,
    },
}

/// Compiles a legacy record's literal endpoint IDs into live endpoint views. Legacy records
/// keep their wire shape; this is the private solver/adapter representation for the same ID.
pub fn compile_legacy(
    document: &ProjectDoc,
    constraint: &crate::model::Constraint,
) -> Result<FramedConstraint, String> {
    if let crate::model::Constraint::Framed {
        id,
        source,
        target,
        relation,
    } = constraint
    {
        return Ok(FramedConstraint::Framed {
            id: id.clone(),
            source: source.clone(),
            target: target.clone(),
            relation: relation.clone(),
        });
    }

    let source = legacy_endpoint(document, constraint.id(), Side::Source, constraint.source())?;
    let target = legacy_endpoint(document, constraint.id(), Side::Target, constraint.target())?;
    Ok(FramedConstraint::Framed {
        id: constraint.id().to_owned(),
        source,
        target,
        relation: ConstraintRelation::from_legacy(constraint),
    })
}

fn legacy_endpoint(
    document: &ProjectDoc,
    constraint_id: &str,
    side: Side,
    part_id: &str,
) -> Result<ConstraintEndpoint, String> {
    let located =
        |message: String| format!("Constraint {constraint_id} {}: {message}", side.name());
    let has_part = document.parts.iter().any(|part| part.id == part_id);
    let pairs = document
        .hardware
        .as_ref()
        .map(|hardware| hardware.pairs.as_slice())
        .unwrap_or_default();
    let mut matched = None;
    for pair in pairs {
        let Some(halves) = &pair.halves else {
            continue;
        };
        for correspondence in halves.correspondences.iter().filter(|correspondence| {
            correspondence.kind == MirroredItemKind::Part && correspondence.mirror_id == part_id
        }) {
            if matched.is_some() {
                return Err(located(format!(
                    "part {part_id} belongs to more than one mirrored pair"
                )));
            }
            matched = Some((pair, halves, correspondence));
        }
    }
    if let Some((pair, halves, correspondence)) = matched {
        if let Some(exception) = halves.overlay_of(correspondence) {
            return match exception.state {
                MirroredExceptionState::Local if has_part => Ok(ConstraintEndpoint::Part {
                    part_id: part_id.to_owned(),
                }),
                MirroredExceptionState::Local => Err(located(format!(
                    "references a missing Local part {part_id}"
                ))),
                MirroredExceptionState::Omitted => Err(located(format!(
                    "part {part_id} is omitted in pair {}",
                    pair.id
                ))),
            };
        }
        if !has_part
            || !document
                .parts
                .iter()
                .any(|part| part.id == correspondence.source_id)
        {
            return Err(located(format!(
                "references a missing linked endpoint {part_id}"
            )));
        }
        return Ok(ConstraintEndpoint::LinkedMirror {
            pair_id: pair.id.clone(),
            source_part_id: correspondence.source_id.clone(),
            mirror_part_id: part_id.to_owned(),
        });
    }
    if has_part {
        Ok(ConstraintEndpoint::Part {
            part_id: part_id.to_owned(),
        })
    } else {
        Err(located(format!("references a missing part {part_id}")))
    }
}

/// How a mirrored companion sits against its mirrored key (ADR-0010).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompanionRule {
    /// Reflect the companion's pose like the key's.
    Literal,
    /// Keep the source's world heading; the position is reflected.
    Stabiliser,
    /// Keep the source's key-local position and heading.
    Duplicate,
    /// Reflect, then add `(u, v)` in the mirrored key's frame and `w` to the heading.
    Tweaked { u: f64, v: f64, w: f64 },
}

/// How a companion rule turns the authoritative heading into the visible one, and back.
#[derive(Clone, Copy)]
enum Heading {
    /// The world heading is kept.
    Keep,
    /// `mapping - c`, plus a tweak when the rule has one.
    Reflect { mapping: f64, tweak: Option<f64> },
    /// The heading is turned by a fixed amount.
    Turn(f64),
}

impl Heading {
    fn forward(self, heading: f64) -> f64 {
        match self {
            Self::Keep => heading,
            Self::Reflect {
                mapping,
                tweak: None,
            } => mapping - heading,
            Self::Reflect {
                mapping,
                tweak: Some(tweak),
            } => (mapping - heading) + tweak,
            Self::Turn(turn) => heading + turn,
        }
    }

    fn inverse(self, heading: f64) -> f64 {
        match self {
            Self::Keep => heading,
            Self::Reflect {
                mapping,
                tweak: None,
            } => mapping - heading,
            Self::Reflect {
                mapping,
                tweak: Some(tweak),
            } => (mapping + tweak) - heading,
            Self::Turn(turn) => heading - turn,
        }
    }
}

/// A companion rule written once as a map from the authoritative pose to the visible one, so
/// `project` and `unproject` cannot drift apart.
///
/// Position: with no `turn`, `forward(mapping, p)` plus `shift` when the rule has one; with a
/// `turn`, `origin + R(turn) (p - pivot)`. Heading: see [`Heading`].
///
/// Why each rule keeps its own operation order: these numbers are persisted and hashed once a
/// framed record is stored, so the order is part of the contract and is pinned bit for bit by
/// `companion_rules_are_bit_pinned_to_the_arithmetic_that_was_reviewed`. Do not "tidy" a sign
/// or fold a zero shift in: a stabiliser's `-0.0` heading and a literal position must stay
/// as the reviewed rule wrote them.
struct CompanionMap {
    turn: Option<f64>,
    pivot: Vec2,
    origin: Vec2,
    shift: Option<Vec2>,
    heading: Heading,
}

impl CompanionRule {
    /// The rule's map for a pair `mapping` and the resolved key pose it is measured against.
    fn frame(self, mapping: Pose2, key: Pose2) -> CompanionMap {
        let reflected = |shift: Option<Vec2>, heading: Heading| CompanionMap {
            turn: None,
            pivot: Vec2::default(),
            origin: Vec2::default(),
            shift,
            heading,
        };
        match self {
            Self::Literal => reflected(
                None,
                Heading::Reflect {
                    mapping: mapping.rotation,
                    tweak: None,
                },
            ),
            Self::Stabiliser => reflected(None, Heading::Keep),
            Self::Tweaked { u, v, w } => reflected(
                Some(turned(Vec2 { x: u, y: v }, mapping.rotation - key.rotation)),
                Heading::Reflect {
                    mapping: mapping.rotation,
                    tweak: Some(w),
                },
            ),
            Self::Duplicate => {
                let turn = mapping.rotation - 2.0 * key.rotation;
                CompanionMap {
                    turn: Some(turn),
                    pivot: key.at,
                    origin: pair::forward(mapping, key.at),
                    shift: None,
                    heading: Heading::Turn(turn),
                }
            }
        }
    }
}

/// A live endpoint frame. Linked mappings are supplied by the current pair owner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EndpointFrame {
    Identity,
    /// An independent mirrored part: the full heading is turned half a turn.
    LinkedMirror {
        mapping: Pose2,
    },
    /// A controller mirrored part: the heading is the plain reflection, without the half
    /// turn, matching the part projection rule.
    LinkedController {
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
    pub fn with_key(self, resolved: Option<Pose2>) -> Self {
        match (self, resolved) {
            (Self::LinkedCompanion { mapping, rule, .. }, Some(key)) => Self::LinkedCompanion {
                mapping,
                key: Some(key),
                rule,
            },
            (frame, _) => frame,
        }
    }

    pub fn project(self, authoritative: Pose2) -> Result<Pose2, String> {
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
            Self::LinkedController { mapping } | Self::LinkedKey { mapping } => {
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
                        match map.shift {
                            Some(shift) => Vec2 {
                                x: reflected.x + shift.x,
                                y: reflected.y + shift.y,
                            },
                            None => reflected,
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
                    rotation: map.heading.forward(authoritative.rotation),
                }
            }
        };
        validate_pose(projected)?;
        Ok(projected)
    }

    pub fn unproject(self, visible: Pose2, previous_authoritative: Pose2) -> Result<Pose2, String> {
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
            Self::LinkedKey { mapping } | Self::LinkedController { mapping } => {
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
                        match map.shift {
                            Some(shift) => Vec2 {
                                x: visible.at.x - shift.x,
                                y: visible.at.y - shift.y,
                            },
                            None => visible.at,
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
                let rotation = map.heading.inverse(visible.rotation);
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
pub enum Side {
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
pub struct ResolvedEndpoint {
    pub visible_id: String,
    pub authoritative_id: String,
    /// For a companion frame, `key` is a placeholder until the solver supplies `owner`'s pose.
    pub frame: EndpointFrame,
    /// The authoritative key a companion endpoint is projected against. The solver resolves it
    /// first, so the projection depends on the key's live pose and a cycle through it is refused.
    pub owner: Option<String>,
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
    // Controllers keep the reflected heading without the half turn, like the part
    // projection rule, so relations authored against them resolve in the same frame
    // the mirror is reconciled with.
    let is_controller = document
        .parts
        .iter()
        .find(|part| part.id == source_part_id)
        .is_some_and(|part| {
            document.definitions.iter().any(|definition| {
                definition.id == part.definition_id && definition.kind == PartKind::Controller
            })
        });
    if is_controller {
        return Ok((EndpointFrame::LinkedController { mapping }, None));
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
pub(crate) fn authoritative_part<'a>(document: &'a ProjectDoc, part_id: &'a str) -> &'a str {
    derived_from(document, part_id).map_or(part_id, |(_, correspondence)| {
        correspondence.source_id.as_str()
    })
}

/// Binds one authored endpoint to the document as it stands. Nothing is cached: a later
/// pair map, correspondence or exception change is read again on the next resolution.
pub fn resolve_endpoint(
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
pub fn solve_target(
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
