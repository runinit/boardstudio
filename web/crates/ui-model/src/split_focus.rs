//! Ticket 14: view-only editing focus for split boards (Layout only).
//!
//! The half without focus draws dimmed (variant B: grey dashed outline with an
//! inline click-to-edit label). Focus starts on the source (Left) half, moves
//! only on canvas clicks of the dimmed half, and never writes the document.

use crate::tree::TreeContext;
use boardstudio_core::model::ProjectDoc;

/// Which split half has editing focus. `Left` is the source half.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SplitHalf {
    #[default]
    Left,
    Right,
}

impl SplitHalf {
    pub fn other(self) -> Self {
        match self {
            SplitHalf::Left => SplitHalf::Right,
            SplitHalf::Right => SplitHalf::Left,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SplitHalf::Left => "Left half",
            SplitHalf::Right => "Right half",
        }
    }

    /// Inline canvas label for the dimmed half (prototype variant B).
    pub fn dim_label(self) -> String {
        format!("{} · linked mirror · click to edit here", self.label())
    }
}

/// The split axis, if any layout carries a mirror link.
pub fn split_axis_x(document: &ProjectDoc) -> Option<f64> {
    document
        .layouts
        .iter()
        .find_map(|layout| layout.mirror_link.as_ref().map(|link| link.axis_x))
}

/// Whether the document has a linked split pair at all. Same definition as the
/// tree's half grouping: a link exists, or a layout is a link source.
pub fn has_split(document: &ProjectDoc) -> bool {
    document
        .layouts
        .iter()
        .any(|layout| layout.mirror_link.is_some())
        || document.layouts.iter().any(|layout| {
            document.layouts.iter().any(|other| {
                other
                    .mirror_link
                    .as_ref()
                    .is_some_and(|link| link.source_id == layout.id)
            })
        })
}

fn matrix_origin_x(document: &ProjectDoc, id: &str) -> Option<f64> {
    document
        .matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .map(|matrix| matrix.origin.x)
}

fn layout_half(document: &ProjectDoc, axis: f64, matrix_ids: &[String]) -> Option<SplitHalf> {
    if matrix_ids.is_empty() {
        return None;
    }
    let mean = matrix_ids
        .iter()
        .filter_map(|id| matrix_origin_x(document, id))
        .sum::<f64>()
        / (matrix_ids.len().max(1) as f64);
    Some(if mean < axis {
        SplitHalf::Left
    } else {
        SplitHalf::Right
    })
}

/// Which half a matrix belongs to, by its owning layout's mean origin against
/// the split axis. Mirrors the tree's Left/Right half grouping.
pub fn matrix_half(document: &ProjectDoc, matrix_id: &str) -> Option<SplitHalf> {
    let axis = split_axis_x(document)?;
    let owner = document
        .layouts
        .iter()
        .find(|layout| layout.matrix_ids.iter().any(|id| id == matrix_id))?;
    layout_half(document, axis, &owner.matrix_ids)
}

/// Which half a part belongs to: its matrix half for matrix members, else its
/// owning layout's half.
pub fn part_half(document: &ProjectDoc, part_id: &str) -> Option<SplitHalf> {
    let axis = split_axis_x(document)?;
    if let Some(matrix) = document
        .matrices
        .iter()
        .find(|matrix| matrix.part_ids.iter().any(|id| id == part_id))
    {
        return matrix_half(document, &matrix.id);
    }
    let owner = document
        .layouts
        .iter()
        .find(|layout| layout.part_ids.iter().any(|id| id == part_id))?;
    layout_half(document, axis, &owner.matrix_ids).or_else(|| {
        let x = document
            .parts
            .iter()
            .find(|part| part.id == part_id)
            .map(|part| part.pose.at.x)?;
        Some(if x < axis {
            SplitHalf::Left
        } else {
            SplitHalf::Right
        })
    })
}

/// Which half a tree selection context belongs to, if any.
pub fn context_half(document: &ProjectDoc, context: &TreeContext) -> Option<SplitHalf> {
    match context {
        TreeContext::Matrix { matrix_id }
        | TreeContext::Row { matrix_id, .. }
        | TreeContext::Column { matrix_id, .. }
        | TreeContext::Key { matrix_id, .. } => matrix_half(document, matrix_id),
        TreeContext::Component {
            part_id: Some(part_id),
            ..
        } => part_half(document, part_id),
        TreeContext::Component {
            matrix_id: Some(matrix_id),
            ..
        } => matrix_half(document, matrix_id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{
        Board, Layout, LayoutMirrorLink, Matrix, MirroredCorrespondence, MirroredItemKind, Part,
        Pose2, Side, Vec2,
    };

    fn matrix(id: &str, origin_x: f64, part_ids: Vec<&str>) -> Matrix {
        serde_json::from_value(serde_json::json!({
            "id": id, "rows": 1, "columns": 1,
            "pitch": {"x": 19.0, "y": 19.0}, "origin": {"x": origin_x, "y": 0.0},
            "definitionId": "switch:base", "partIds": part_ids, "boardId": "board",
            "cells": [{"row": 0, "column": 0, "enabled": true, "assemblies": []}]
        }))
        .unwrap()
    }

    fn part(id: &str, x: f64) -> Part {
        Part {
            keycap: None,
            outline: None,
            id: id.into(),
            definition_id: "switch:base".into(),
            reference: id.to_uppercase(),
            pose: Pose2 {
                at: Vec2 { x, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            anchor: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn split_document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("split-focus", "Split focus");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["matrix/matrix/r0c0".into(), "matrix/paired/r0c0".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document
            .matrices
            .push(matrix("matrix", -30.0, vec!["matrix/matrix/r0c0"]));
        document
            .matrices
            .push(matrix("paired", 30.0, vec!["matrix/paired/r0c0"]));
        document.parts.push(part("matrix/matrix/r0c0", -30.0));
        document.parts.push(part("matrix/paired/r0c0", 30.0));
        document.layouts.push(Layout {
            id: "source-layout".into(),
            name: "Left half".into(),
            board_id: "board".into(),
            matrix_ids: vec!["matrix".into()],
            part_ids: vec![],
            mirror_link: None,
        });
        document.layouts.push(Layout {
            id: "paired-layout".into(),
            name: "Right half".into(),
            board_id: "board".into(),
            matrix_ids: vec!["paired".into()],
            part_ids: vec![],
            mirror_link: Some(LayoutMirrorLink {
                correspondences: vec![MirroredCorrespondence {
                    companion_mode: None,
                    companion_tweak: None,
                    kind: MirroredItemKind::Matrix,
                    source_id: "matrix".into(),
                    mirror_id: "paired".into(),
                }],
                exceptions: vec![],
                source_id: "source-layout".into(),
                axis_x: 0.0,
                split_gap: None,
                spacing: None,
            }),
        });
        document
    }

    #[test]
    fn focus_starts_on_the_source_half_with_variant_b_labels() {
        assert_eq!(SplitHalf::default(), SplitHalf::Left);
        assert_eq!(SplitHalf::Left.other(), SplitHalf::Right);
        assert_eq!(SplitHalf::Right.other(), SplitHalf::Left);
        assert_eq!(
            SplitHalf::Right.dim_label(),
            "Right half · linked mirror · click to edit here"
        );
        assert_eq!(
            SplitHalf::Left.dim_label(),
            "Left half · linked mirror · click to edit here"
        );
    }

    #[test]
    fn split_halves_resolve_from_the_mirror_link_axis() {
        let document = split_document();
        assert_eq!(split_axis_x(&document), Some(0.0));
        assert!(has_split(&document));
        assert_eq!(matrix_half(&document, "matrix"), Some(SplitHalf::Left));
        assert_eq!(matrix_half(&document, "paired"), Some(SplitHalf::Right));
        assert_eq!(matrix_half(&document, "missing"), None);
        assert_eq!(
            part_half(&document, "matrix/matrix/r0c0"),
            Some(SplitHalf::Left)
        );
        assert_eq!(
            part_half(&document, "matrix/paired/r0c0"),
            Some(SplitHalf::Right)
        );
        assert_eq!(part_half(&document, "missing"), None);
    }

    #[test]
    fn contexts_resolve_to_their_half_and_neutral_contexts_resolve_to_none() {
        let document = split_document();
        assert_eq!(
            context_half(
                &document,
                &TreeContext::Key {
                    matrix_id: "paired".into(),
                    row: 0,
                    column: 0
                }
            ),
            Some(SplitHalf::Right)
        );
        assert_eq!(
            context_half(
                &document,
                &TreeContext::Matrix {
                    matrix_id: "matrix".into()
                }
            ),
            Some(SplitHalf::Left)
        );
        assert_eq!(
            context_half(
                &document,
                &TreeContext::Component {
                    part_id: Some("matrix/paired/r0c0".into()),
                    matrix_id: None,
                    row: None,
                    column: None,
                    assembly_id: None,
                }
            ),
            Some(SplitHalf::Right)
        );
        assert_eq!(
            context_half(
                &document,
                &TreeContext::Board {
                    board_id: "board".into()
                }
            ),
            None
        );
    }

    #[test]
    fn documents_without_a_mirror_link_have_no_halves() {
        let mut document = split_document();
        document
            .layouts
            .retain(|layout| layout.mirror_link.is_none());
        assert_eq!(split_axis_x(&document), None);
        assert!(!has_split(&document));
        assert_eq!(matrix_half(&document, "matrix"), None);
    }
}
