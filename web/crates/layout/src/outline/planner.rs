//! Pure planning for outline lifecycle actions.

use crate::outline_settings::{OutlineEdit, apply_outline_edit, generated_feature};
use boardstudio_application::{AcceptedSnapshot, OperationId, Scope};
use boardstudio_core::model::{
    EditOperation, EditPhase, Operation, OutlineConnection, OutlineContourEdit,
    OutlineControlPoint, OutlineFeature, Part, Side, Vec2,
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
/// Why an action plans to no edit: its target is gone or no longer eligible, or the value
/// is already what is accepted.
pub(crate) enum Skip {
    Retire(String),
    Unchanged,
}

fn retire<T>(reason: &str) -> Result<T, Skip> {
    Err(Skip::Retire(reason.to_owned()))
}

fn next_edited_outline_name(state: Option<&boardstudio_core::model::BoardOutline>) -> String {
    let next_number = state
        .into_iter()
        .flat_map(|state| &state.versions)
        .filter_map(|version| {
            version
                .name
                .strip_prefix("Edited outline ")
                .and_then(|number| number.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    format!("Edited outline {next_number}")
}

fn all_version_ids(document: &boardstudio_core::model::ProjectDoc) -> Vec<String> {
    document
        .board_outlines
        .iter()
        .flat_map(|state| &state.versions)
        .map(|version| version.id.clone())
        .collect()
}

/// Plan one outline action against an accepted snapshot: the edit operation and its target
/// ids, built from the accepted document so a queued action applies on top of the edits
/// accepted before it. New identities are chosen here, against the accepted document, from a
/// seed captured at submit, so two queued adds cannot collide.
pub(crate) fn plan_action(
    snapshot: &AcceptedSnapshot,
    action: &OutlineAction,
    seed: u64,
) -> Result<(EditOperation, Vec<String>), Skip> {
    let document = snapshot.document.as_ref();
    let (_, _, _, _, board_id) = action.envelope();
    let Some(board) = document.boards.iter().find(|board| board.id == *board_id) else {
        return retire("The board no longer exists.");
    };
    let state = document
        .board_outlines
        .iter()
        .find(|state| state.board_id == *board_id);
    let generated = generated_feature(document, board);
    let active_version = state.and_then(|state| state.active_version_id.as_deref());
    let version_of = |version_id: &str| {
        state
            .into_iter()
            .flat_map(|state| &state.versions)
            .find(|version| version.id == version_id)
    };
    match action {
        OutlineAction::Activate { version_id, .. } => {
            if version_id
                .as_deref()
                .is_some_and(|id| version_of(id).is_none())
            {
                return retire("That outline version no longer exists.");
            }
            if active_version == version_id.as_deref() {
                return Err(Skip::Unchanged);
            }
            Ok((
                EditOperation::SelectOutline {
                    board_id: board_id.clone(),
                    version_id: version_id.clone(),
                },
                vec![board_id.clone()],
            ))
        }
        OutlineAction::Copy { .. } => {
            let version_id = unique_outline_version_id(seed, all_version_ids(document));
            Ok((
                EditOperation::CopyOutline {
                    board_id: board_id.clone(),
                    version_id,
                    name: next_edited_outline_name(state),
                    edit: None,
                    feature: None,
                },
                vec![board_id.clone()],
            ))
        }
        OutlineAction::Delete { version_id, .. } => {
            if active_version != Some(version_id.as_str()) || version_of(version_id).is_none() {
                return retire("That outline version no longer exists.");
            }
            Ok((
                EditOperation::RemoveOutline {
                    board_id: board_id.clone(),
                    version_id: version_id.clone(),
                },
                vec![board_id.clone()],
            ))
        }
        OutlineAction::AddFeature {
            context, feature, ..
        } => {
            if let TreeContext::OutlineVersion { version_id, .. } = context
                && active_version != version_id.as_deref()
            {
                return retire("The outline version changed before the feature was added.");
            }
            let OutlineFeature::Polygon {
                id,
                points,
                anchor_part_id,
                operation,
            } = feature
            else {
                return retire("Only polygons can be added here.");
            };
            if points.len() < 3
                || points
                    .iter()
                    .any(|point| !point.x.is_finite() || !point.y.is_finite())
            {
                return retire("A polygon needs at least three finite points.");
            }
            let existing_ids = document
                .outline
                .iter()
                .map(|item| item.id().to_owned())
                .chain(
                    document
                        .board_outlines
                        .iter()
                        .flat_map(|item| &item.versions)
                        .flat_map(|version| &version.geometry.features)
                        .map(|item| item.id().to_owned()),
                )
                .collect::<Vec<_>>();
            let id = if existing_ids.contains(id) {
                unique_outline_entity_id("outline-feature", seed, existing_ids)
            } else {
                id.clone()
            };
            let feature = OutlineFeature::Polygon {
                id: id.clone(),
                points: points.clone(),
                anchor_part_id: anchor_part_id.clone(),
                operation: *operation,
            };
            if let Some(version_id) = active_version {
                let mut replacement = document.clone();
                let Some(version) = replacement
                    .board_outlines
                    .iter_mut()
                    .find(|outline| outline.board_id == *board_id)
                    .and_then(|outline| {
                        outline
                            .versions
                            .iter_mut()
                            .find(|version| version.id == version_id)
                    })
                else {
                    return retire("The outline version no longer exists.");
                };
                version.geometry.features.push(feature);
                Ok((
                    EditOperation::ReplaceDocument {
                        document: Box::new(replacement),
                    },
                    vec![board_id.clone(), id],
                ))
            } else {
                Ok((
                    EditOperation::CopyOutline {
                        board_id: board_id.clone(),
                        version_id: unique_outline_version_id(seed, all_version_ids(document)),
                        name: next_edited_outline_name(state),
                        edit: None,
                        feature: Some(feature),
                    },
                    vec![board_id.clone(), id],
                ))
            }
        }
        OutlineAction::AddConnection { points, .. } => {
            if active_version.is_some() {
                return retire("Connections only apply to the automatic outline.");
            }
            if points.len() < 2
                || points
                    .iter()
                    .any(|point| !point.x.is_finite() || !point.y.is_finite())
            {
                return retire("A connection needs at least two finite points.");
            }
            let Some(OutlineFeature::PartEnvelope {
                part_ids, settings, ..
            }) = generated
            else {
                return retire("The automatic outline no longer exists.");
            };
            let eligible = document
                .parts
                .iter()
                .filter(|part| {
                    part_ids.contains(&part.id)
                        && !part
                            .outline
                            .as_ref()
                            .is_some_and(|outline| outline.excluded)
                })
                .collect::<Vec<_>>();
            let mut feature = generated.expect("matched generated feature").clone();
            let OutlineFeature::PartEnvelope { connections, .. } = &mut feature else {
                return retire("The automatic outline no longer exists.");
            };
            connections.push(OutlineConnection {
                id: unique_outline_entity_id(
                    "outline-connection",
                    seed,
                    connections.iter().map(|connection| connection.id.clone()),
                ),
                width: settings.bridge_width,
                points: points
                    .iter()
                    .enumerate()
                    .map(|(index, point)| {
                        if index == 0 || index + 1 == points.len() {
                            outline_connection_control(*point, &eligible)
                        } else {
                            OutlineControlPoint {
                                at: *point,
                                part_id: None,
                            }
                        }
                    })
                    .collect(),
            });
            let id = feature.id().to_owned();
            Ok((
                EditOperation::SetOutline { feature },
                vec![board_id.clone(), id],
            ))
        }
        OutlineAction::SetFeature {
            version_id,
            before,
            after,
            ..
        } => {
            if before.id() != after.id() {
                return retire("The outline feature identity changed.");
            }
            if active_version != version_id.as_deref() {
                return retire("The outline version changed before the feature was edited.");
            }
            let accepted = if let Some(version_id) = version_id {
                version_of(version_id).and_then(|version| {
                    version
                        .geometry
                        .features
                        .iter()
                        .find(|feature| feature.id() == before.id())
                })
            } else {
                document.outline.iter().find(|feature| {
                    feature.id() == before.id()
                        && board.outline_ids.iter().any(|id| id == feature.id())
                })
            };
            let Some(accepted) = accepted else {
                return retire("The outline feature no longer exists.");
            };
            if accepted == after {
                return Err(Skip::Unchanged);
            }
            let operation = if let Some(version_id) = version_id {
                let mut replacement = document.clone();
                let Some(feature) = replacement
                    .board_outlines
                    .iter_mut()
                    .find(|outline| outline.board_id == *board_id)
                    .and_then(|outline| {
                        outline
                            .versions
                            .iter_mut()
                            .find(|version| version.id == *version_id)
                    })
                    .and_then(|version| {
                        version
                            .geometry
                            .features
                            .iter_mut()
                            .find(|feature| feature.id() == before.id())
                    })
                else {
                    return retire("The outline feature no longer exists.");
                };
                *feature = after.clone();
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                }
            } else {
                EditOperation::SetOutline {
                    feature: after.clone(),
                }
            };
            Ok((operation, vec![board_id.clone(), before.id().to_owned()]))
        }
        OutlineAction::RemoveFeature {
            version_id,
            feature_id,
            ..
        } => {
            if active_version != Some(version_id.as_str()) {
                return retire("The outline version changed before the feature was removed.");
            }
            let Some(version) = version_of(version_id) else {
                return retire("The outline version no longer exists.");
            };
            let authored_index = version
                .geometry
                .features
                .iter()
                .filter(|feature| !matches!(feature, OutlineFeature::PartEnvelope { .. }))
                .position(|feature| feature.id() == feature_id);
            match authored_index {
                None => return retire("The outline feature no longer exists."),
                Some(0) => return retire("The base outline feature cannot be removed."),
                Some(_) => {}
            }
            let mut replacement = document.clone();
            let Some(target) = replacement
                .board_outlines
                .iter_mut()
                .find(|outline| outline.board_id == *board_id)
                .and_then(|outline| {
                    outline
                        .versions
                        .iter_mut()
                        .find(|version| version.id == *version_id)
                })
            else {
                return retire("The outline version no longer exists.");
            };
            target
                .geometry
                .features
                .retain(|feature| feature.id() != feature_id);
            Ok((
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
                vec![board_id.clone(), feature_id.clone()],
            ))
        }
        OutlineAction::FocusGap { .. } => retire("A gap focus is not an edit."),
        OutlineAction::Update { edit, .. } => {
            let Some((operation, target_ids)) =
                apply_outline_edit(document, &snapshot.scene, board_id, edit, OperationId(seed))
            else {
                if let OutlineEdit::RenameVersion { version_id, .. } = edit
                    && version_of(version_id).is_none()
                {
                    return retire("That outline version no longer exists.");
                }
                return Err(Skip::Unchanged);
            };
            Ok((operation, target_ids))
        }
        OutlineAction::EditPerimeter { target, points, .. } => {
            if points.len() < 3
                || points
                    .iter()
                    .any(|point| !point.x.is_finite() || !point.y.is_finite())
            {
                return retire("A perimeter needs at least three finite points.");
            }
            match target {
                OutlinePointTarget::Generated { contour } => {
                    let Some(source_points) = snapshot
                        .scene
                        .board_outline_scenes
                        .iter()
                        .find(|scene| scene.board_id == *board_id)
                        .and_then(|scene| scene.source_contours.get(*contour as usize))
                        .map(|source| &source.points)
                    else {
                        return retire("The perimeter no longer exists.");
                    };
                    if active_version.is_some() {
                        return retire(
                            "The outline version changed before the perimeter was edited.",
                        );
                    }
                    if source_points == points {
                        return Err(Skip::Unchanged);
                    }
                    Ok((
                        EditOperation::CopyOutline {
                            board_id: board_id.clone(),
                            version_id: unique_outline_version_id(seed, all_version_ids(document)),
                            name: next_edited_outline_name(state),
                            edit: Some(OutlineContourEdit {
                                contour: *contour,
                                points: points.clone(),
                            }),
                            feature: None,
                        },
                        vec![board_id.clone()],
                    ))
                }
                OutlinePointTarget::Fixed {
                    version_id,
                    feature_id,
                    anchor_part_id,
                    operation,
                } => {
                    if active_version != Some(version_id.as_str()) {
                        return retire(
                            "The outline version changed before the perimeter was edited.",
                        );
                    }
                    let Some(OutlineFeature::Polygon {
                        id,
                        points: current,
                        anchor_part_id: current_anchor,
                        operation: current_operation,
                    }) = version_of(version_id).and_then(|version| {
                        version
                            .geometry
                            .features
                            .iter()
                            .find(|feature| feature.id() == *feature_id)
                    })
                    else {
                        return retire("The outline feature no longer exists.");
                    };
                    if current_anchor != anchor_part_id || current_operation != operation {
                        return retire(
                            "The outline feature changed before the perimeter was edited.",
                        );
                    }
                    if current == points {
                        return Err(Skip::Unchanged);
                    }
                    Ok((
                        EditOperation::SetOutline {
                            feature: OutlineFeature::Polygon {
                                id: id.clone(),
                                points: points.clone(),
                                anchor_part_id: anchor_part_id.clone(),
                                operation: *operation,
                            },
                        },
                        vec![board_id.clone(), feature_id.clone()],
                    ))
                }
            }
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../outline_lifecycle_tests.rs"]
mod lifecycle_tests;
