//! Pure geometry helpers and the presentation outline action vocabulary.
//!
//! Every action here is turned into one typed Core outline edit by the layout
//! crate's outline adapter; no planning or replacement document happens in this module. The
//! unique-id helpers below feed the adapter for the version and connection ids it still
//! chooses itself; Core validates them.

use crate::outline_settings::OutlineEdit;
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_model::model::{
    EditPhase, Operation, OutlineControlPoint, OutlineFeature, Part, Side, Vec2,
};
use boardstudio_web_ui_model::tree::TreeContext;

pub fn unique_outline_entity_id(
    prefix: &str,
    operation_id: u64,
    existing_ids: impl IntoIterator<Item = String>,
) -> String {
    let existing_ids = existing_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    let base = format!("{prefix}-{operation_id}");
    if !existing_ids.contains(&base) {
        return base;
    }

    for suffix in 2usize.. {
        let candidate = format!("{base}-{suffix}");
        if !existing_ids.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!("a finite saved ID set cannot exhaust the suffix sequence")
}

pub fn unique_outline_version_id(
    operation_id: u64,
    existing_ids: impl IntoIterator<Item = String>,
) -> String {
    unique_outline_entity_id("outline-version", operation_id, existing_ids)
}

/// The (scope, token, revision, generation, board id) envelope carried by every
/// presentation outline action. `OutlineActionKind` holds only the payload that
/// differs per action, so the envelope is written once instead of twelve times.
#[derive(Clone, Debug, PartialEq)]
pub struct OutlineActionEnvelope {
    pub scope: Scope,
    pub token: boardstudio_application::SnapshotToken,
    pub revision: u64,
    pub generation: u64,
    pub board_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OutlineActionKind {
    Activate {
        version_id: Option<String>,
    },
    Copy,
    Delete {
        version_id: String,
    },
    Update {
        edit: OutlineEdit,
    },
    Refine,
    Freeze {
        version_id: String,
    },
    EditPerimeter {
        target: OutlinePointTarget,
        points: Vec<Vec2>,
        phase: EditPhase,
        transaction_id: String,
    },
    AddFeature {
        context: TreeContext,
        feature: OutlineFeature,
    },
    AddConnection {
        points: Vec<Vec2>,
    },
    SetFeature {
        version_id: Option<String>,
        before: OutlineFeature,
        after: OutlineFeature,
    },
    RemoveFeature {
        version_id: String,
        feature_id: String,
    },
    FocusGap {
        gap_id: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutlineAction {
    pub envelope: OutlineActionEnvelope,
    pub kind: OutlineActionKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OutlinePointTarget {
    Generated {
        contour: u32,
    },
    Fixed {
        version_id: String,
        feature_id: String,
        anchor_part_id: Option<String>,
        operation: Operation,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct EditablePerimeter {
    pub target: OutlinePointTarget,
    pub points: Vec<Vec2>,
    pub canvas_points: Vec<Vec2>,
    pub anchor: Option<PerimeterAnchor>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerimeterAnchor {
    pub at: Vec2,
    pub rotation: f64,
    pub back: bool,
}

impl PerimeterAnchor {
    pub fn world(self, point: Vec2) -> Vec2 {
        let x = point.x * if self.back { -1.0 } else { 1.0 };
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        Vec2 {
            x: self.at.x + x * cos - point.y * sin,
            y: self.at.y + x * sin + point.y * cos,
        }
    }

    pub fn local(self, point: Vec2) -> Vec2 {
        let (sin, cos) = (-self.rotation).to_radians().sin_cos();
        let x = point.x - self.at.x;
        let y = point.y - self.at.y;
        Vec2 {
            x: (x * cos - y * sin) * if self.back { -1.0 } else { 1.0 },
            y: x * sin + y * cos,
        }
    }
}

pub fn outline_connection_control(at: Vec2, eligible: &[&Part]) -> OutlineControlPoint {
    let nearest = eligible.iter().copied().min_by(|left, right| {
        let distance = |part: &Part| (part.pose.at.x - at.x).hypot(part.pose.at.y - at.y);
        distance(left).total_cmp(&distance(right))
    });
    if let Some(part) =
        nearest.filter(|part| (part.pose.at.x - at.x).hypot(part.pose.at.y - at.y) <= 10.0)
    {
        let anchor = PerimeterAnchor {
            at: part.pose.at,
            rotation: part.pose.rotation,
            back: part.side == Side::Back,
        };
        OutlineControlPoint {
            at: anchor.local(at),
            part_id: Some(part.id.clone()),
        }
    } else {
        OutlineControlPoint { at, part_id: None }
    }
}

pub fn connection_point_world(point: &OutlineControlPoint, parts: &[Part]) -> Vec2 {
    point
        .part_id
        .as_deref()
        .and_then(|id| parts.iter().find(|part| part.id == id))
        .map(|part| {
            PerimeterAnchor {
                at: part.pose.at,
                rotation: part.pose.rotation,
                back: part.side == Side::Back,
            }
            .world(point.at)
        })
        .unwrap_or(point.at)
}

pub fn move_connection_point(
    feature: &OutlineFeature,
    connection_id: &str,
    point_index: usize,
    world: Vec2,
    parts: &[Part],
) -> OutlineFeature {
    let mut next = feature.clone();
    if let OutlineFeature::PartEnvelope { connections, .. } = &mut next
        && let Some(point) = connections
            .iter_mut()
            .find(|connection| connection.id == connection_id)
            .and_then(|connection| connection.points.get_mut(point_index))
    {
        point.at = point
            .part_id
            .as_deref()
            .and_then(|id| parts.iter().find(|part| part.id == id))
            .map(|part| {
                PerimeterAnchor {
                    at: part.pose.at,
                    rotation: part.pose.rotation,
                    back: part.side == Side::Back,
                }
                .local(world)
            })
            .unwrap_or(world);
    }
    next
}

pub fn feature_anchor(part_id: Option<&str>, parts: &[Part]) -> Option<PerimeterAnchor> {
    part_id
        .and_then(|id| parts.iter().find(|part| part.id == id))
        .map(|part| PerimeterAnchor {
            at: part.pose.at,
            rotation: part.pose.rotation,
            back: part.side == Side::Back,
        })
}

pub fn attach_connection_point(
    feature: &OutlineFeature,
    connection_id: &str,
    point_index: usize,
    next_part_id: Option<&str>,
    parts: &[Part],
) -> OutlineFeature {
    let mut next = feature.clone();
    if let OutlineFeature::PartEnvelope { connections, .. } = &mut next
        && let Some(point) = connections
            .iter_mut()
            .find(|connection| connection.id == connection_id)
            .and_then(|connection| connection.points.get_mut(point_index))
    {
        let world = connection_point_world(point, parts);
        point.at = feature_anchor(next_part_id, parts).map_or(world, |anchor| anchor.local(world));
        point.part_id = next_part_id.map(str::to_owned);
    }
    next
}

impl OutlineAction {
    pub fn envelope(
        &self,
    ) -> (
        &Scope,
        &boardstudio_application::SnapshotToken,
        u64,
        u64,
        &String,
    ) {
        let envelope = &self.envelope;
        (
            &envelope.scope,
            &envelope.token,
            envelope.revision,
            envelope.generation,
            &envelope.board_id,
        )
    }

    pub fn for_tree(
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        generation: u64,
        context: &TreeContext,
    ) -> Option<Self> {
        let TreeContext::OutlineVersion {
            board_id,
            version_id,
        } = context
        else {
            return None;
        };
        Some(Self {
            envelope: OutlineActionEnvelope {
                scope: scope.clone(),
                token: snapshot.token,
                revision: snapshot.document.revision,
                generation,
                board_id: board_id.clone(),
            },
            kind: OutlineActionKind::Activate {
                version_id: version_id.clone(),
            },
        })
    }
}
