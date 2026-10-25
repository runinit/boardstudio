//! Inverse mapping of outline operations and their reviewed geometry (ticket 113).
//!
//! Outline versions are authored board values. A mirror-side edit is translated as one request:
//! version/feature identities, anchored references, geometry, winding and reviewed contours all
//! travel through the same pair context. Lifecycle changes keep their captured seed and user
//! values; explicit identities are converted with the same whole-board-token rule as projection.

use super::{Canonical, Links, Target};
use crate::mechanical::pair;
use crate::model::{
    Contour, EditOperation, MirroredHalves, MirroredItemKind, OutlineConnection,
    OutlineContourEdit, OutlineControlPoint, OutlineFeature, OutlineSettings, ProtectedOutlineGap,
    Vec2,
};
use crate::outline_edit::{OutlineChange, OutlineEditTarget};

struct Context<'a> {
    mapping: crate::model::Pose2,
    source_board: &'a str,
    mirror_board: &'a str,
    halves: &'a MirroredHalves,
    matrix_owned_parts: std::collections::BTreeSet<String>,
    controller_parts: std::collections::BTreeSet<String>,
}

impl<'a> Context<'a> {
    fn new(
        document: &'a crate::model::ProjectDoc,
        mapping: crate::model::Pose2,
        halves: &'a MirroredHalves,
    ) -> Self {
        let mirror_board = &halves.mirror_board_id;
        let matrix_owned_parts = super::super::matrix_owned_parts(document, mirror_board);
        let controller_parts = super::super::controller_parts(document, mirror_board);
        Self {
            mapping,
            source_board: &halves.source_board_id,
            mirror_board,
            halves,
            matrix_owned_parts,
            controller_parts,
        }
    }

    fn correspondence(&self, id: &str, kind: MirroredItemKind) -> Option<String> {
        self.halves
            .correspondences
            .iter()
            .find(|item| item.kind == kind && item.mirror_id == id)
            .map(|item| item.source_id.clone())
    }

    fn inverse_identity(&self, id: &str, kind: MirroredItemKind) -> String {
        self.correspondence(id, kind)
            .unwrap_or_else(|| inverse_token(self.mirror_board, self.source_board, id))
    }

    fn part(&self, id: &str) -> String {
        self.correspondence(id, MirroredItemKind::Part)
            .unwrap_or_else(|| id.to_owned())
    }

    fn outline(&self, id: &str) -> String {
        self.inverse_identity(id, MirroredItemKind::Outline)
    }

    fn version(&self, id: &str) -> String {
        self.inverse_identity(id, MirroredItemKind::OutlineVersion)
    }

    fn identity(&self, id: &str) -> String {
        let outline = self.outline(id);
        if outline != id {
            return outline;
        }
        let version = self.version(id);
        if version != id {
            return version;
        }
        inverse_token(self.mirror_board, self.source_board, id)
    }

    fn full_rule_part(&self, id: &str) -> bool {
        !self.matrix_owned_parts.contains(id) && !self.controller_parts.contains(id)
    }

    fn anchored(&self, point: Vec2, part_id: &str) -> Vec2 {
        if self.full_rule_part(part_id) {
            Vec2 {
                x: point.x,
                y: -point.y,
            }
        } else {
            Vec2 {
                x: -point.x,
                y: point.y,
            }
        }
    }

    fn control(&self, point: &OutlineControlPoint) -> OutlineControlPoint {
        match &point.part_id {
            Some(part_id) => OutlineControlPoint {
                at: self.anchored(point.at, part_id),
                part_id: Some(self.part(part_id)),
            },
            None => OutlineControlPoint {
                at: pair::inverse(self.mapping, point.at),
                part_id: None,
            },
        }
    }

    fn feature(&self, feature: &OutlineFeature) -> OutlineFeature {
        match feature {
            OutlineFeature::Polygon {
                anchor_part_id,
                id,
                points,
                operation,
            } => {
                let anchor = anchor_part_id.as_deref();
                OutlineFeature::Polygon {
                    anchor_part_id: anchor.map(|part| self.part(part)),
                    id: self.identity(id),
                    points: points
                        .iter()
                        .rev()
                        .map(|point| match anchor {
                            Some(part) => self.anchored(*point, part),
                            None => pair::inverse(self.mapping, *point),
                        })
                        .collect(),
                    operation: *operation,
                }
            }
            OutlineFeature::Rect {
                rotation,
                anchor_part_id,
                id,
                center,
                size,
                radius,
                operation,
            } => {
                let anchored = anchor_part_id.is_some();
                OutlineFeature::Rect {
                    rotation: if anchored {
                        rotation.map(|angle| -angle)
                    } else {
                        (rotation.is_some() || self.mapping.rotation != 0.0)
                            .then(|| pair::inverse_angle(self.mapping, rotation.unwrap_or(0.0)))
                    },
                    anchor_part_id: anchor_part_id.as_deref().map(|part| self.part(part)),
                    id: self.identity(id),
                    center: match anchor_part_id.as_deref() {
                        Some(part) => self.anchored(*center, part),
                        None => pair::inverse(self.mapping, *center),
                    },
                    size: *size,
                    radius: *radius,
                    operation: *operation,
                }
            }
            OutlineFeature::PartEnvelope {
                connections,
                settings,
                id,
                part_ids,
                margin,
                operation,
            } => OutlineFeature::PartEnvelope {
                connections: connections
                    .iter()
                    .map(|connection| self.connection(connection))
                    .collect(),
                settings: self.settings(settings),
                id: self.identity(id),
                part_ids: part_ids.iter().map(|part| self.part(part)).collect(),
                margin: *margin,
                operation: *operation,
            },
        }
    }

    fn connection(&self, connection: &OutlineConnection) -> OutlineConnection {
        OutlineConnection {
            id: self.identity(&connection.id),
            width: connection.width,
            points: connection
                .points
                .iter()
                .map(|point| self.control(point))
                .collect(),
        }
    }

    fn gap(&self, gap: &ProtectedOutlineGap) -> ProtectedOutlineGap {
        ProtectedOutlineGap {
            id: self.identity(&gap.id),
            points: gap.points.iter().map(|point| self.control(point)).collect(),
        }
    }

    fn settings(&self, settings: &OutlineSettings) -> OutlineSettings {
        let mut settings = settings.clone();
        if let Some(repair) = settings.repair.as_mut() {
            repair.keep_gaps = repair.keep_gaps.iter().map(|gap| self.gap(gap)).collect();
        }
        settings
    }

    fn points(&self, points: &[Vec2]) -> Vec<Vec2> {
        points
            .iter()
            .rev()
            .map(|point| pair::inverse(self.mapping, *point))
            .collect()
    }
}

fn inverse_token(from: &str, to: &str, id: &str) -> String {
    if let Some(swapped) = super::super::substitute_token(id, from, to) {
        swapped
    } else if let Some(rest) = id.strip_prefix(&format!("{from}/")) {
        format!("{to}/{rest}")
    } else {
        id.to_owned()
    }
}

fn board_context<'a>(links: &Links<'a>, board_id: &str) -> Result<Option<Context<'a>>, String> {
    let mut found = None;
    for pair in &links.pairs {
        let halves = pair.halves;
        if halves.mirror_board_id == board_id {
            if found.is_some() {
                return Err(format!(
                    "Board {board_id:?} is the mirror of more than one pair"
                ));
            }
            found = Some(Context::new(links.document, pair.mapping, halves));
        }
    }
    Ok(found)
}

fn mapped_board<'a>(
    links: &Links<'a>,
    board_id: &str,
) -> Result<(String, Option<Context<'a>>), String> {
    let Some(context) = board_context(links, board_id)? else {
        return Ok((board_id.to_owned(), None));
    };
    Ok((context.source_board.to_owned(), Some(context)))
}

pub(super) fn translate(
    links: &Links<'_>,
    operation: &EditOperation,
    canonical: &mut Canonical,
) -> Result<(), String> {
    use EditOperation as Op;
    match operation {
        Op::SetOutline { feature } => {
            let Some(context) = feature_context(links, feature.id())? else {
                return Ok(());
            };
            canonical.operation = Op::SetOutline {
                feature: context.feature(feature),
            };
        }
        Op::CopyOutline {
            board_id,
            version_id,
            name,
            edit,
            feature,
        } => {
            let (board_id, context) = mapped_board(links, board_id)?;
            let Some(context) = context else {
                return Ok(());
            };
            canonical.operation = Op::CopyOutline {
                board_id,
                version_id: context.version(version_id),
                name: name.clone(),
                edit: edit.as_ref().map(|edit| OutlineContourEdit {
                    contour: edit.contour,
                    points: context.points(&edit.points),
                }),
                feature: feature.as_ref().map(|feature| context.feature(feature)),
            };
        }
        Op::SelectOutline {
            board_id,
            version_id,
        } => {
            if let Some(id) = version_id {
                let Some(context) = version_context(links, id)? else {
                    return Ok(());
                };
                if context.mirror_board != board_id {
                    return Err(format!(
                        "Outline version {id:?} belongs to a different mirrored board"
                    ));
                }
                canonical.operation = Op::SelectOutline {
                    board_id: context.source_board.to_owned(),
                    version_id: Some(context.version(id)),
                };
                return Ok(());
            }
            let (board_id, _context) = mapped_board(links, board_id)?;
            canonical.operation = Op::SelectOutline {
                board_id,
                version_id: None,
            };
        }
        Op::RenameOutline {
            board_id,
            version_id,
            name,
        } => {
            let Some(context) = version_context(links, version_id)? else {
                return Ok(());
            };
            if context.mirror_board != board_id {
                return Err(format!(
                    "Outline version {version_id:?} belongs to a different mirrored board"
                ));
            }
            canonical.operation = Op::RenameOutline {
                board_id: context.source_board.to_owned(),
                version_id: context.version(version_id),
                name: name.clone(),
            };
        }
        Op::RemoveOutline {
            board_id,
            version_id,
        } => {
            let Some(context) = version_context(links, version_id)? else {
                return Ok(());
            };
            if context.mirror_board != board_id {
                return Err(format!(
                    "Outline version {version_id:?} belongs to a different mirrored board"
                ));
            }
            canonical.operation = Op::RemoveOutline {
                board_id: context.source_board.to_owned(),
                version_id: context.version(version_id),
            };
        }
        Op::ApplyOutlineEdit { edit } => {
            let mut edit = (**edit).clone();
            let board_id = edit.target.board_id().to_owned();
            if let OutlineEditTarget::Version { version_id, .. } = edit.target.clone() {
                let Some(context) = version_context(links, &version_id)? else {
                    return Ok(());
                };
                if context.mirror_board != board_id {
                    return Err(format!(
                        "Outline version {version_id:?} belongs to a different mirrored board"
                    ));
                }
                edit.target = OutlineEditTarget::Version {
                    board_id: context.source_board.to_owned(),
                    version_id: context.version(&version_id),
                };
                edit.change = change(&context, edit.change);
                edit.reviewed = edit.reviewed.as_ref().map(|contours| {
                    reviewed(links, &context, &edit.target, version_id.as_str(), contours)
                });
                canonical.operation = Op::ApplyOutlineEdit {
                    edit: Box::new(edit),
                };
                return Ok(());
            }
            let (source_board, context) = mapped_board(links, &board_id)?;
            let Some(context) = context else {
                return Ok(());
            };
            edit.target = OutlineEditTarget::Board {
                board_id: source_board,
            };
            edit.change = change(&context, edit.change);
            edit.reviewed = edit
                .reviewed
                .as_ref()
                .map(|contours| reviewed(links, &context, &edit.target, "", contours));
            canonical.operation = Op::ApplyOutlineEdit {
                edit: Box::new(edit),
            };
        }
        _ => unreachable!("outline translator only receives outline operations"),
    }
    Ok(())
}

fn feature_context<'a>(links: &'a Links<'a>, id: &str) -> Result<Option<Context<'a>>, String> {
    match links.target(MirroredItemKind::Outline, id)? {
        Target::NotMirror | Target::Local => Ok(None),
        Target::Omitted => Err(format!(
            "{id:?} is omitted on its mirror half and has no geometry; rejoin it before editing it"
        )),
        Target::Linked(link) => Ok(Some(Context::new(
            links.document,
            link.pair.mapping,
            link.pair.halves,
        ))),
    }
}

fn version_context<'a>(links: &'a Links<'a>, id: &str) -> Result<Option<Context<'a>>, String> {
    match links.target(MirroredItemKind::OutlineVersion, id)? {
        Target::NotMirror | Target::Local => Ok(None),
        Target::Omitted => Err(format!(
            "Outline version {id:?} is omitted on its mirror half and has no geometry; rejoin it before editing it"
        )),
        Target::Linked(link) => Ok(Some(Context::new(
            links.document,
            link.pair.mapping,
            link.pair.halves,
        ))),
    }
}

/// Keep exact review equality stable for a review captured from the current mirror projection.
/// Floating point forward/inverse transforms can otherwise change a source coordinate by a few
/// ulps and retire a valid edit. A stale review is still transformed as authored, so it remains
/// unequal to the accepted source shape and keeps the established retirement behavior.
fn reviewed(
    links: &Links<'_>,
    context: &Context<'_>,
    translated_target: &OutlineEditTarget,
    mirror_version_id: &str,
    contours: &[Contour],
) -> Vec<Contour> {
    if let Some((source, mirror)) = current_review_shapes(
        links.document,
        context,
        translated_target,
        mirror_version_id,
    ) && mirror == contours
    {
        return source;
    }
    contours
        .iter()
        .map(|contour| Contour {
            points: context.points(&contour.points),
            hole: contour.hole,
        })
        .collect()
}

fn current_review_shapes(
    document: &crate::model::ProjectDoc,
    context: &Context<'_>,
    target: &OutlineEditTarget,
    mirror_version_id: &str,
) -> Option<(Vec<Contour>, Vec<Contour>)> {
    let (cache, _, _) = crate::geometry::outlines(document, None, &[]);
    match target {
        OutlineEditTarget::Board { .. } => {
            let source_board = document
                .boards
                .iter()
                .find(|board| board.id == context.source_board)?;
            let mirror_board = document
                .boards
                .iter()
                .find(|board| board.id == context.mirror_board)?;
            Some((
                crate::geometry::generated_source(document, &cache, source_board).source,
                crate::geometry::generated_source(document, &cache, mirror_board).source,
            ))
        }
        OutlineEditTarget::Version { version_id, .. } => {
            let source = document
                .board_outlines
                .iter()
                .find(|state| state.board_id == context.source_board)?
                .versions
                .iter()
                .find(|version| version.id == *version_id)?;
            let mirror = document
                .board_outlines
                .iter()
                .find(|state| state.board_id == context.mirror_board)?
                .versions
                .iter()
                .find(|version| version.id == mirror_version_id)?;
            Some((
                version_review_shape(document, &cache, context.source_board, source),
                version_review_shape(document, &cache, context.mirror_board, mirror),
            ))
        }
    }
}

fn version_review_shape(
    document: &crate::model::ProjectDoc,
    cache: &crate::geometry::OutlineCache,
    board_id: &str,
    version: &crate::model::OutlineVersion,
) -> Vec<Contour> {
    let features = if version.ownership.is_fixed() {
        version.geometry.features.clone()
    } else {
        crate::outline_versions::resolved_features(document, board_id, version)
    };
    crate::geometry::feature_source_contours(document, cache, &features)
}

fn change(context: &Context<'_>, change: OutlineChange) -> OutlineChange {
    use OutlineChange as C;
    match change {
        C::SetMargin { margin } => C::SetMargin { margin },
        C::SetCorners { corners } => C::SetCorners { corners },
        C::SetSize { size } => C::SetSize { size },
        C::SetBridgeWidth { bridge_width } => C::SetBridgeWidth { bridge_width },
        C::SetRepairEnabled { enabled } => C::SetRepairEnabled { enabled },
        C::SetMaximumGapSpan { value } => C::SetMaximumGapSpan { value },
        C::SetMinimumConnectionWidth { value } => C::SetMinimumConnectionWidth { value },
        C::SetEdgeClearance { value } => C::SetEdgeClearance { value },
        C::SetProtectedGap { gap_id, points } => C::SetProtectedGap {
            gap_id: context.identity(&gap_id),
            points: points.iter().map(|point| context.control(point)).collect(),
        },
        C::RemoveProtectedGap { gap_id } => C::RemoveProtectedGap {
            gap_id: context.identity(&gap_id),
        },
        C::AddFeature { feature } => C::AddFeature {
            feature: context.feature(&feature),
        },
        C::SetFeature { feature } => C::SetFeature {
            feature: context.feature(&feature),
        },
        C::RemoveFeature { feature_id } => C::RemoveFeature {
            feature_id: context.identity(&feature_id),
        },
        C::AddConnection { connection } => C::AddConnection {
            connection: context.connection(&connection),
        },
        C::RemoveConnection { connection_id } => C::RemoveConnection {
            connection_id: context.identity(&connection_id),
        },
        C::CreateGenerated {
            margin,
            settings,
            part_ids,
        } => C::CreateGenerated {
            margin,
            settings: context.settings(&settings),
            part_ids: part_ids.iter().map(|part| context.part(part)).collect(),
        },
        C::CreateLinkedRefinement { version_id, name } => C::CreateLinkedRefinement {
            version_id: context.version(&version_id),
            name,
        },
        C::Copy { version_id, name } => C::Copy {
            version_id: context.version(&version_id),
            name,
        },
        C::Freeze { name } => C::Freeze { name },
        C::EditGeneratedPerimeter { contour, points } => C::EditGeneratedPerimeter {
            contour,
            points: context.points(&points),
        },
        C::EditFeaturePerimeter { feature_id, points } => C::EditFeaturePerimeter {
            feature_id: context.identity(&feature_id),
            points: context.points(&points),
        },
        C::Activate => C::Activate,
        C::Rename { name } => C::Rename { name },
        C::Delete => C::Delete,
    }
}
