//! Pure geometry helpers and the presentation outline action vocabulary.
//!
//! Every action here is turned into one typed Core outline edit by
//! [`super::adapter`]; no planning or replacement document happens in this module. The
//! unique-id helpers below feed the adapter for the version and connection ids it still
//! chooses itself; Core validates them.

use crate::outline_settings::OutlineEdit;
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::model::{
    EditPhase, Operation, OutlineControlPoint, OutlineFeature, Part, Side, Vec2,
};
use boardstudio_web_ui_model::tree::TreeContext;

pub(super) fn unique_outline_entity_id(
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

pub(super) fn unique_outline_version_id(
    operation_id: u64,
    existing_ids: impl IntoIterator<Item = String>,
) -> String {
    unique_outline_entity_id("outline-version", operation_id, existing_ids)
}

#[derive(Clone, Debug, PartialEq)]
pub enum OutlineAction {
    Activate {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        context: TreeContext,
        require_selected_version: bool,
        board_id: String,
        version_id: Option<String>,
    },
    Copy {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
    },
    Delete {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        version_id: String,
    },
    Update {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        edit: OutlineEdit,
    },
    Refine {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
    },
    Freeze {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        version_id: String,
    },
    EditPerimeter {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: TreeContext,
        target: OutlinePointTarget,
        points: Vec<Vec2>,
        phase: EditPhase,
        transaction_id: String,
    },
    AddFeature {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: TreeContext,
        feature: OutlineFeature,
    },
    AddConnection {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: TreeContext,
        points: Vec<Vec2>,
    },
    SetFeature {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: TreeContext,
        version_id: Option<String>,
        before: OutlineFeature,
        after: OutlineFeature,
    },
    RemoveFeature {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: TreeContext,
        version_id: String,
        feature_id: String,
    },
    FocusGap {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: TreeContext,
        gap_id: String,
    },
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
pub(super) struct EditablePerimeter {
    pub(super) target: OutlinePointTarget,
    pub(super) points: Vec<Vec2>,
    pub(super) canvas_points: Vec<Vec2>,
    pub(super) anchor: Option<PerimeterAnchor>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PerimeterAnchor {
    pub(super) at: Vec2,
    pub(super) rotation: f64,
    pub(super) back: bool,
}

impl PerimeterAnchor {
    pub(super) fn world(self, point: Vec2) -> Vec2 {
        let x = point.x * if self.back { -1.0 } else { 1.0 };
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        Vec2 {
            x: self.at.x + x * cos - point.y * sin,
            y: self.at.y + x * sin + point.y * cos,
        }
    }

    pub(super) fn local(self, point: Vec2) -> Vec2 {
        let (sin, cos) = (-self.rotation).to_radians().sin_cos();
        let x = point.x - self.at.x;
        let y = point.y - self.at.y;
        Vec2 {
            x: (x * cos - y * sin) * if self.back { -1.0 } else { 1.0 },
            y: x * sin + y * cos,
        }
    }
}

pub(super) fn outline_connection_control(at: Vec2, eligible: &[&Part]) -> OutlineControlPoint {
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

pub(super) fn connection_point_world(point: &OutlineControlPoint, parts: &[Part]) -> Vec2 {
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

pub(super) fn move_connection_point(
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

pub(super) fn feature_anchor(part_id: Option<&str>, parts: &[Part]) -> Option<PerimeterAnchor> {
    part_id
        .and_then(|id| parts.iter().find(|part| part.id == id))
        .map(|part| PerimeterAnchor {
            at: part.pose.at,
            rotation: part.pose.rotation,
            back: part.side == Side::Back,
        })
}

pub(super) fn attach_connection_point(
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
    pub(super) fn envelope(
        &self,
    ) -> (
        &Scope,
        &boardstudio_application::SnapshotToken,
        u64,
        u64,
        &String,
    ) {
        match self {
            OutlineAction::Activate {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::Copy {
                scope,
                token,
                revision,
                generation,
                board_id,
            }
            | OutlineAction::Delete {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::Update {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::Refine {
                scope,
                token,
                revision,
                generation,
                board_id,
            }
            | OutlineAction::Freeze {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::EditPerimeter {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::AddFeature {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::AddConnection {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::SetFeature {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::RemoveFeature {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::FocusGap {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            } => (scope, token, *revision, *generation, board_id),
        }
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
        Some(Self::Activate {
            scope: scope.clone(),
            token: snapshot.token,
            revision: snapshot.document.revision,
            generation,
            context: context.clone(),
            require_selected_version: true,
            board_id: board_id.clone(),
            version_id: version_id.clone(),
        })
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../outline_lifecycle_tests.rs"]
mod lifecycle_tests;
