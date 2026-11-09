//! Ticket 08: the pure Canvas Anchor pick candidate projection.
//!
//! The rail Anchor token arms pick mode for an anchored matrix or a part.
//! This module is the pure seam: it takes the accepted document, Core's
//! evaluated matrix scenes and the selection, and returns the committable
//! anchor-key candidates plus the major targets with their kind and current
//! flag. Loop-forming targets are excluded (no ring); every other Core
//! refusal (missing, cross-board/Layout, constraint-driven) still commits
//! through the existing resolvers so Core's message surfaces.
//!
//! Contract gap (flagged, not silently expanded): Core anchors target keys
//! only (`MatrixAnchor`/`PartAnchor` name a matrix key). The prototype's
//! major targets (parts, the half origin) are returned with
//! `supported: false` and never commit through `SetMatrixAnchor` /
//! `SetPartAnchor`; the half origin only marks the current placement of a
//! detached part or an unanchored matrix.

use boardstudio_core::CoreEngine;
use boardstudio_core::evaluated_part_pose;
use boardstudio_core::model::{
    CoreReply, CoreRequest, EditOperation, Follow, MatrixAnchor, MatrixScene, PartAnchor, Pose2,
    ProjectDoc, Vec2,
};
use boardstudio_web_ui_model::tree::TreeContext;

/// A committable anchor-key candidate: one live key in the owner's Layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchorKeyCandidate {
    pub matrix_id: String,
    pub row: u32,
    pub column: u32,
    /// True when this key is the owner's current anchor target.
    pub current: bool,
}

/// A prototype major target (a part or the half origin): display-only.
/// `supported` is always false under the current Core contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchorMajorTarget {
    pub kind: AnchorMajorKind,
    /// True when the owner is currently placed from this target.
    pub current: bool,
    /// Always false: no Core op commits these targets.
    pub supported: bool,
}

/// The kind of a major target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnchorMajorKind {
    Part(String),
    HalfOrigin,
}

/// The pick projection for one selection.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AnchorPickCandidates {
    /// Committable keys: same board and Layout, loop-forming excluded.
    pub keys: Vec<AnchorKeyCandidate>,
    /// Display-only majors (parts, half origin), never committed.
    pub majors: Vec<AnchorMajorTarget>,
}

/// Ticket 08 ring geometry, in screen px (prototype variant D).
/// Candidate keys draw a violet ring 4.5 px across (6 px on hover) with a
/// 24 px hit area; the current anchor fills a white dot.
pub const CANDIDATE_RING_PX: f64 = 4.5;
/// Candidate ring on hover, in screen px.
pub const CANDIDATE_RING_HOVER_PX: f64 = 6.0;
/// Candidate hit area edge, in screen px.
pub const CANDIDATE_HIT_PX: f64 = 24.0;
/// Major targets (parts, half origin) draw a 7.5 px ring with a cross
/// (9 px on hover) and a 36 px hit area; labels stay on.
pub const MAJOR_RING_PX: f64 = 7.5;
/// Major ring on hover, in screen px.
pub const MAJOR_RING_HOVER_PX: f64 = 9.0;
/// Major hit area edge, in screen px.
pub const MAJOR_HIT_PX: f64 = 36.0;

/// Project the pick candidates for `selection`. Returns `None` when the
/// selection cannot arm pick mode (unanchored matrices, locked or
/// member/companion parts, unknown owners). Takes the scenes beside the
/// document for seam parity with `anchor_badge`; key liveness reads the
/// document rows/columns.
pub fn anchor_pick_candidates(
    document: &ProjectDoc,
    _scenes: &[&MatrixScene],
    selection: &TreeContext,
) -> Option<AnchorPickCandidates> {
    enum Owner {
        Matrix {
            matrix_id: String,
            current: Vec<(String, u32, u32)>,
        },
        Part {
            part_id: String,
            current: Option<(String, u32, u32)>,
        },
    }

    // Resolve the owner and its current anchor target(s). Unanchored
    // matrices, locked parts and matrix members/companions never arm.
    let (owner, layout_id, anchored) = match selection {
        TreeContext::Matrix { matrix_id } => {
            let matrix = document
                .matrices
                .iter()
                .find(|matrix| &matrix.id == matrix_id)?;
            let anchor = matrix.anchor.as_ref()?;
            let layout = document
                .layouts
                .iter()
                .find(|layout| layout.matrix_ids.iter().any(|id| id == matrix_id))?;
            let current = anchor
                .targets()
                .into_iter()
                .map(|part| (part.matrix_id, part.row, part.column))
                .collect();
            (
                Owner::Matrix {
                    matrix_id: matrix_id.clone(),
                    current,
                },
                layout.id.clone(),
                true,
            )
        }
        TreeContext::Component {
            part_id: Some(part_id),
            matrix_id: None,
            ..
        } => {
            let part = document.parts.iter().find(|part| &part.id == part_id)?;
            if part.locked == Some(true) {
                return None;
            }
            if document
                .matrices
                .iter()
                .any(|matrix| matrix.part_ids.contains(part_id))
            {
                return None;
            }
            let layout = document
                .layouts
                .iter()
                .find(|layout| layout.part_ids.iter().any(|id| id == part_id))?;
            let current = part
                .anchor
                .as_ref()
                .map(|anchor| (anchor.matrix_id.clone(), anchor.row, anchor.column));
            let anchored = current.is_some();
            (
                Owner::Part {
                    part_id: part_id.clone(),
                    current,
                },
                layout.id.clone(),
                anchored,
            )
        }
        _ => return None,
    };
    let _ = anchored;

    let layout = document
        .layouts
        .iter()
        .find(|layout| layout.id == layout_id)?;
    let board_id = layout.board_id.clone();

    // Forward anchor walk: every matrix the target's chain resolves through.
    // A candidate whose chain reaches the owner would close a loop.
    let reaches_owner = |target_matrix: &str, owner_id: &str| -> bool {
        let mut stack = vec![target_matrix.to_owned()];
        let mut seen = std::collections::BTreeSet::new();
        while let Some(cursor) = stack.pop() {
            if cursor == owner_id {
                return true;
            }
            if !seen.insert(cursor.clone()) {
                continue;
            }
            if let Some(next) = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == cursor)
                .and_then(|matrix| matrix.anchor.as_ref())
            {
                stack.extend(next.targets().iter().map(|part| part.matrix_id.clone()));
            }
        }
        false
    };

    let owner_id = match &owner {
        Owner::Matrix { matrix_id, .. } => matrix_id.clone(),
        Owner::Part { .. } => String::new(),
    };
    let is_matrix_owner = matches!(owner, Owner::Matrix { .. });

    let mut keys = Vec::new();
    for matrix_id in &layout.matrix_ids {
        let Some(target) = document
            .matrices
            .iter()
            .find(|matrix| &matrix.id == matrix_id)
        else {
            continue;
        };
        let same_board = target
            .board_id
            .as_deref()
            .is_none_or(|board| board == board_id);
        if !same_board {
            continue;
        }
        // Loop-forming targets show no ring. A matrix owner excludes its
        // own keys and every matrix whose anchor chain reaches it; part
        // anchors cannot cycle, so part picks keep every layout key.
        if is_matrix_owner && reaches_owner(&target.id, &owner_id) {
            continue;
        }
        for row in 0..target.rows {
            for column in 0..target.columns {
                let current = match &owner {
                    Owner::Matrix { current, .. } => current
                        .iter()
                        .any(|(matrix, r, c)| matrix == &target.id && *r == row && *c == column),
                    Owner::Part { current, .. } => {
                        current.as_ref().is_some_and(|(matrix, r, c)| {
                            matrix == &target.id && *r == row && *c == column
                        })
                    }
                };
                keys.push(AnchorKeyCandidate {
                    matrix_id: target.id.clone(),
                    row,
                    column,
                    current,
                });
            }
        }
    }

    // Prototype majors: the layout's other independent parts plus the half
    // origin. Display-only under the current Core contract (`supported`
    // stays false); the half origin marks the current placement while
    // detached.
    let owner_part_id = match &owner {
        Owner::Part { part_id, .. } => Some(part_id.clone()),
        Owner::Matrix { .. } => None,
    };
    let mut majors: Vec<AnchorMajorTarget> = layout
        .part_ids
        .iter()
        .filter(|id| owner_part_id.as_deref() != Some(id.as_str()))
        .filter(|id| {
            !document
                .matrices
                .iter()
                .any(|matrix| matrix.part_ids.iter().any(|member| member == *id))
        })
        .map(|id| AnchorMajorTarget {
            kind: AnchorMajorKind::Part(id.clone()),
            current: false,
            supported: false,
        })
        .collect();
    let detached = match &owner {
        Owner::Matrix { current, .. } => current.is_empty(),
        Owner::Part { current, .. } => current.is_none(),
    };
    majors.push(AnchorMajorTarget {
        kind: AnchorMajorKind::HalfOrigin,
        current: detached,
        supported: false,
    });

    Some(AnchorPickCandidates { keys, majors })
}

/// A committable pick target: one live key in the owner's Layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickTarget {
    pub matrix_id: String,
    pub row: u32,
    pub column: u32,
}

/// A validated pick commit: the single Core op plus the toast labels.
/// The op keeps the owner's Follow subset (and drops an aggregate to a
/// single key, like the Inspector's Pick); part values ship zeroed because
/// Core carries the stored values (`set_anchor`).
#[derive(Clone, Debug, PartialEq)]
pub struct PickCommit {
    pub operation: EditOperation,
    pub item_label: String,
    pub old_anchor_label: String,
    pub new_anchor_label: String,
}

/// The matrix display name: its name, else its id.
pub fn matrix_display_name(document: &ProjectDoc, matrix_id: &str) -> String {
    document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .and_then(|matrix| matrix.name.clone())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| matrix_id.to_owned())
}

/// One anchor key in words: `Main R1C1` (1-based, like the badge).
pub fn anchor_key_label(document: &ProjectDoc, matrix_id: &str, row: u32, column: u32) -> String {
    format!(
        "{} R{}C{}",
        matrix_display_name(document, matrix_id),
        row + 1,
        column + 1
    )
}

/// The part display name: its reference, else its id.
fn part_display_name(document: &ProjectDoc, part_id: &str) -> String {
    document
        .parts
        .iter()
        .find(|part| part.id == part_id)
        .map(|part| {
            if part.reference.trim().is_empty() {
                part.id.clone()
            } else {
                part.reference.clone()
            }
        })
        .unwrap_or_else(|| part_id.to_owned())
}

fn full_follow() -> Follow {
    Follow {
        x: true,
        y: true,
        rotation: true,
    }
}

/// Validate one pick against the live projection: the target must be a
/// committable key (loop-forming targets have no ring) and differ from the
/// current anchor. Returns `None` for anything that must not commit.
pub fn pick_commit(
    document: &ProjectDoc,
    selection: &TreeContext,
    target: &PickTarget,
) -> Option<PickCommit> {
    let projected = anchor_pick_candidates(document, &[], selection)?;
    let candidate = projected.keys.iter().find(|key| {
        key.matrix_id == target.matrix_id && key.row == target.row && key.column == target.column
    })?;
    if candidate.current {
        return None;
    }
    let new_anchor_label = anchor_key_label(document, &target.matrix_id, target.row, target.column);
    match selection {
        TreeContext::Matrix { matrix_id } => {
            let matrix = document
                .matrices
                .iter()
                .find(|matrix| &matrix.id == matrix_id)?;
            let current = matrix.anchor.as_ref()?;
            let old_anchor_label =
                anchor_key_label(document, &current.matrix_id, current.row, current.column);
            Some(PickCommit {
                operation: EditOperation::SetMatrixAnchor {
                    matrix_id: matrix_id.clone(),
                    anchor: MatrixAnchor {
                        matrix_id: target.matrix_id.clone(),
                        row: target.row,
                        column: target.column,
                        follow: current.follow,
                        aggregate: None,
                    },
                },
                item_label: matrix_display_name(document, matrix_id),
                old_anchor_label,
                new_anchor_label,
            })
        }
        TreeContext::Component {
            part_id: Some(part_id),
            matrix_id: None,
            ..
        } => {
            let part = document.parts.iter().find(|part| &part.id == part_id)?;
            let follow = part
                .anchor
                .as_ref()
                .map(|anchor| anchor.follow)
                .unwrap_or_else(full_follow);
            let old_anchor_label = part
                .anchor
                .as_ref()
                .map(|anchor| {
                    anchor_key_label(document, &anchor.matrix_id, anchor.row, anchor.column)
                })
                .unwrap_or_else(|| "half origin".to_owned());
            Some(PickCommit {
                operation: EditOperation::SetPartAnchor {
                    part_id: part_id.clone(),
                    anchor: PartAnchor {
                        matrix_id: target.matrix_id.clone(),
                        row: target.row,
                        column: target.column,
                        follow,
                        spread: 0.0,
                        stagger: 0.0,
                        splay: 0.0,
                        offset: Vec2::default(),
                        rotation: 0.0,
                    },
                },
                item_label: part_display_name(document, part_id),
                old_anchor_label,
                new_anchor_label,
            })
        }
        _ => None,
    }
}

/// Whether the owner's live anchor already names the target: the commit
/// landing check behind the toast. Unknown owners read as not landed.
pub fn anchor_is_target(
    document: &ProjectDoc,
    selection: &TreeContext,
    target: &PickTarget,
) -> bool {
    match selection {
        TreeContext::Matrix { matrix_id } => document
            .matrices
            .iter()
            .find(|matrix| &matrix.id == matrix_id)
            .and_then(|matrix| matrix.anchor.as_ref())
            .is_some_and(|anchor| {
                anchor.aggregate.is_none()
                    && anchor.matrix_id == target.matrix_id
                    && anchor.row == target.row
                    && anchor.column == target.column
            }),
        TreeContext::Component {
            part_id: Some(part_id),
            matrix_id: None,
            ..
        } => document
            .parts
            .iter()
            .find(|part| &part.id == part_id)
            .and_then(|part| part.anchor.as_ref())
            .is_some_and(|anchor| {
                anchor.matrix_id == target.matrix_id
                    && anchor.row == target.row
                    && anchor.column == target.column
            }),
        _ => false,
    }
}

/// The amber hover preview: the owner's post-commit poses from Core's own
/// evaluation. The document is cloned, the anchor retargeted on the clone
/// (carried values ride along, exactly like `set_anchor`), and the probe is
/// opened through a scratch engine so the member and part poses materialize
/// through the same pipeline as a commit. No placement math lives here.
pub fn preview_anchor_poses(
    document: &ProjectDoc,
    selection: &TreeContext,
    target: &PickTarget,
) -> Vec<(String, Pose2)> {
    let projected = anchor_pick_candidates(document, &[], selection);
    let Some(projected) = projected else {
        return Vec::new();
    };
    let Some(candidate) = projected.keys.iter().find(|key| {
        key.matrix_id == target.matrix_id && key.row == target.row && key.column == target.column
    }) else {
        return Vec::new();
    };
    if candidate.current {
        return Vec::new();
    }
    let mut probe = document.clone();
    match selection {
        TreeContext::Matrix { matrix_id } => {
            let Some(matrix) = probe
                .matrices
                .iter_mut()
                .find(|matrix| matrix.id == *matrix_id)
            else {
                return Vec::new();
            };
            let Some(current) = matrix.anchor.clone() else {
                return Vec::new();
            };
            matrix.anchor = Some(MatrixAnchor {
                matrix_id: target.matrix_id.clone(),
                row: target.row,
                column: target.column,
                follow: current.follow,
                aggregate: None,
            });
        }
        TreeContext::Component {
            part_id: Some(part_id),
            matrix_id: None,
            ..
        } => {
            let Some(part) = probe.parts.iter_mut().find(|part| part.id == *part_id) else {
                return Vec::new();
            };
            match part.anchor.as_mut() {
                Some(anchor) => {
                    anchor.matrix_id.clone_from(&target.matrix_id);
                    anchor.row = target.row;
                    anchor.column = target.column;
                }
                None => {
                    part.anchor = Some(PartAnchor {
                        matrix_id: target.matrix_id.clone(),
                        row: target.row,
                        column: target.column,
                        follow: full_follow(),
                        spread: 0.0,
                        stagger: 0.0,
                        splay: 0.0,
                        offset: Vec2::default(),
                        rotation: 0.0,
                    });
                }
            }
        }
        _ => return Vec::new(),
    }
    let mut engine = CoreEngine::new();
    let probe = match engine.handle(CoreRequest::Open {
        id: "anchor-pick-preview".into(),
        document: probe,
    }) {
        CoreReply::Scene { document, .. } => *document,
        _ => return Vec::new(),
    };
    match selection {
        TreeContext::Matrix { matrix_id } => {
            let prefix = format!("matrix/{matrix_id}/r");
            probe
                .parts
                .iter()
                .filter(|part| part.id.starts_with(prefix.as_str()))
                .map(|part| (part.id.clone(), evaluated_part_pose(&probe, part)))
                .collect()
        }
        TreeContext::Component {
            part_id: Some(part_id),
            matrix_id: None,
            ..
        } => {
            let Some(part) = probe.parts.iter().find(|part| part.id == *part_id) else {
                return Vec::new();
            };
            vec![(part.id.clone(), evaluated_part_pose(&probe, part))]
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> ProjectDoc {
        let mut document = serde_json::to_value(ProjectDoc::empty("pick", "Pick")).unwrap();
        document["boards"] = serde_json::json!([{ "id": "board", "name": "Board", "outlineIds": [],
            "partIds": [], "netIds": [], "thickness": 1.6 }]);
        document["parts"] = serde_json::json!([
            { "id": "u1", "definitionId": "switch", "reference": "U1",
              "pose": { "at": { "x": -5.0, "y": 2.0 }, "rotation": 0.0 }, "side": "front",
              "anchor": { "matrixId": "main", "row": 0, "column": 0,
                "follow": ["x", "y", "rotation"],
                "spread": 5.0, "stagger": 2.0, "splay": 0.0,
                "offset": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 } },
            { "id": "u2", "definitionId": "switch", "reference": "U2",
              "pose": { "at": { "x": 10.0, "y": 5.0 }, "rotation": 0.0 }, "side": "front" }
        ]);
        document["layouts"] = serde_json::json!([
            { "id": "left", "name": "Left half", "boardId": "board",
              "matrixIds": ["main", "thumb", "looped"], "partIds": ["u1", "u2"] },
            { "id": "right", "name": "Right half", "boardId": "board",
              "matrixIds": [], "partIds": [],
              "mirrorLink": { "sourceId": "left", "axisX": 0.0,
                "splitGap": 40.0, "spacing": 40.0,
                "correspondences": [], "exceptions": [] } }
        ]);
        document["matrices"] = serde_json::json!([
            { "id": "main", "name": "Main", "rows": 2, "columns": 3,
              "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": -10.0, "y": 0.0 },
              "definitionId": "switch", "boardId": "board", "partIds": [],
              "cells": [] },
            { "id": "thumb", "name": "Thumb", "rows": 1, "columns": 1,
              "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": 60.0, "y": 0.0 },
              "definitionId": "switch", "boardId": "board", "partIds": [],
              "anchor": { "matrixId": "main", "row": 0, "column": 0,
                "follow": ["x", "y", "rotation"] },
              "cells": [] },
            { "id": "looped", "name": "Looped", "rows": 1, "columns": 1,
              "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": 80.0, "y": 0.0 },
              "definitionId": "switch", "boardId": "board", "partIds": [],
              "anchor": { "matrixId": "thumb", "row": 0, "column": 0,
                "follow": ["x", "y", "rotation"] },
              "cells": [] }
        ]);
        serde_json::from_value(document).unwrap()
    }

    fn no_scenes() -> Vec<MatrixScene> {
        Vec::new()
    }

    fn refs(scenes: &[MatrixScene]) -> Vec<&MatrixScene> {
        scenes.iter().collect()
    }

    fn candidates(selection: &TreeContext) -> AnchorPickCandidates {
        let document = document();
        let scenes = no_scenes();
        let refs: Vec<&MatrixScene> = scenes.iter().collect();
        anchor_pick_candidates(&document, &refs, selection).expect("the selection arms pick mode")
    }

    #[test]
    fn thumb_matrix_lists_main_keys_marks_current_and_excludes_loops() {
        let projected = candidates(&TreeContext::Matrix {
            matrix_id: "thumb".into(),
        });
        let mut keys: Vec<(String, u32, u32)> = projected
            .keys
            .iter()
            .map(|key| (key.matrix_id.clone(), key.row, key.column))
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                ("main".to_owned(), 0, 0),
                ("main".to_owned(), 0, 1),
                ("main".to_owned(), 0, 2),
                ("main".to_owned(), 1, 0),
                ("main".to_owned(), 1, 1),
                ("main".to_owned(), 1, 2),
            ],
            "the thumb lists the main matrix's keys"
        );
        assert_eq!(
            projected
                .keys
                .iter()
                .filter(|key| key.current)
                .map(|key| (key.matrix_id.clone(), key.row, key.column))
                .collect::<Vec<_>>(),
            vec![("main".to_owned(), 0, 0)],
            "the current anchor key is marked"
        );
        assert!(
            projected
                .keys
                .iter()
                .all(|key| key.matrix_id != "thumb" && key.matrix_id != "looped"),
            "the thumb's own keys and the loop-forming matrix stay out (no ring)"
        );
    }

    #[test]
    fn anchored_part_lists_layout_keys_with_current_anchor() {
        let projected = candidates(&TreeContext::Component {
            part_id: Some("u1".into()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        });
        // main's 6 keys plus the thumb's and the looped matrix's keys: part
        // anchors cannot cycle, so every live layout key stays committable.
        assert_eq!(projected.keys.len(), 8, "layout keys stay committable");
        assert_eq!(
            projected
                .keys
                .iter()
                .filter(|key| key.current)
                .map(|key| (key.matrix_id.clone(), key.row, key.column))
                .collect::<Vec<_>>(),
            vec![("main".to_owned(), 0, 0)],
            "the part's current anchor key is marked"
        );
    }

    #[test]
    fn landing_check_reads_the_live_anchor() {
        let document = document();
        let matrix = TreeContext::Matrix {
            matrix_id: "thumb".into(),
        };
        assert!(
            !anchor_is_target(
                &document,
                &matrix,
                &PickTarget {
                    matrix_id: "main".into(),
                    row: 0,
                    column: 1,
                }
            ),
            "a different key has not landed"
        );
        assert!(
            anchor_is_target(
                &document,
                &matrix,
                &PickTarget {
                    matrix_id: "main".into(),
                    row: 0,
                    column: 0,
                }
            ),
            "the live anchor reads as landed"
        );
    }

    #[test]
    fn unanchored_matrix_arms_no_pick() {
        let document = document();
        let scenes = no_scenes();
        assert!(
            anchor_pick_candidates(
                &document,
                &refs(&scenes),
                &TreeContext::Matrix {
                    matrix_id: "main".into()
                }
            )
            .is_none(),
            "an unanchored matrix has no anchor to retarget"
        );
    }

    #[test]
    fn row_selection_arms_no_pick() {
        let document = document();
        let scenes = no_scenes();
        assert!(
            anchor_pick_candidates(
                &document,
                &refs(&scenes),
                &TreeContext::Row {
                    matrix_id: "main".into(),
                    row: 0
                }
            )
            .is_none(),
            "only matrices and independent parts arm pick mode"
        );
    }

    #[test]
    fn matrix_pick_builds_a_follow_keeping_single_key_op() {
        let document = document();
        let commit = pick_commit(
            &document,
            &TreeContext::Matrix {
                matrix_id: "thumb".into(),
            },
            &PickTarget {
                matrix_id: "main".into(),
                row: 1,
                column: 1,
            },
        )
        .expect("a live key commits");
        assert_eq!(
            commit.operation,
            boardstudio_core::model::EditOperation::SetMatrixAnchor {
                matrix_id: "thumb".into(),
                anchor: boardstudio_core::model::MatrixAnchor {
                    matrix_id: "main".into(),
                    row: 1,
                    column: 1,
                    follow: boardstudio_core::model::Follow {
                        x: true,
                        y: true,
                        rotation: true,
                    },
                    aggregate: None,
                },
            },
            "the pick keeps Follow and drops to a single key"
        );
        assert_eq!(commit.item_label, "Thumb");
        assert_eq!(commit.old_anchor_label, "Main R1C1");
        assert_eq!(commit.new_anchor_label, "Main R2C2");
    }

    #[test]
    fn matrix_pick_of_the_current_anchor_is_a_noop() {
        let document = document();
        assert!(
            pick_commit(
                &document,
                &TreeContext::Matrix {
                    matrix_id: "thumb".into(),
                },
                &PickTarget {
                    matrix_id: "main".into(),
                    row: 0,
                    column: 0,
                },
            )
            .is_none(),
            "clicking the current anchor commits nothing"
        );
    }

    #[test]
    fn matrix_pick_of_a_loop_target_is_a_noop() {
        let document = document();
        assert!(
            pick_commit(
                &document,
                &TreeContext::Matrix {
                    matrix_id: "thumb".into(),
                },
                &PickTarget {
                    matrix_id: "looped".into(),
                    row: 0,
                    column: 0,
                },
            )
            .is_none(),
            "a loop-forming target has no ring and never commits"
        );
    }

    #[test]
    fn part_pick_builds_a_follow_carrying_op_with_zeroed_values() {
        let document = document();
        let commit = pick_commit(
            &document,
            &TreeContext::Component {
                part_id: Some("u1".into()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            },
            &PickTarget {
                matrix_id: "main".into(),
                row: 1,
                column: 2,
            },
        )
        .expect("a live key commits");
        assert_eq!(
            commit.operation,
            boardstudio_core::model::EditOperation::SetPartAnchor {
                part_id: "u1".into(),
                anchor: boardstudio_core::model::PartAnchor {
                    matrix_id: "main".into(),
                    row: 1,
                    column: 2,
                    follow: boardstudio_core::model::Follow {
                        x: true,
                        y: true,
                        rotation: true,
                    },
                    spread: 0.0,
                    stagger: 0.0,
                    splay: 0.0,
                    offset: boardstudio_core::model::Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
            },
            "Core carries the stored values, so the op ships zeroes"
        );
        assert_eq!(commit.item_label, "U1");
        assert_eq!(commit.old_anchor_label, "Main R1C1");
        assert_eq!(commit.new_anchor_label, "Main R2C3");
    }

    #[test]
    fn detached_part_pick_defaults_to_full_follow() {
        let document = document();
        let commit = pick_commit(
            &document,
            &TreeContext::Component {
                part_id: Some("u2".into()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            },
            &PickTarget {
                matrix_id: "main".into(),
                row: 0,
                column: 1,
            },
        )
        .expect("a detached part can anchor");
        let boardstudio_core::model::EditOperation::SetPartAnchor { anchor, .. } = commit.operation
        else {
            panic!("a part pick commits SetPartAnchor");
        };
        assert_eq!(
            anchor.follow,
            boardstudio_core::model::Follow {
                x: true,
                y: true,
                rotation: true,
            }
        );
        assert_eq!(commit.old_anchor_label, "half origin");
    }

    #[test]
    fn detached_part_marks_the_half_origin_current() {
        let projected = candidates(&TreeContext::Component {
            part_id: Some("u2".into()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        });
        assert!(
            !projected.keys.is_empty(),
            "a detached part can still anchor to layout keys"
        );
        assert!(
            projected.keys.iter().all(|key| !key.current),
            "no key is current while detached"
        );
        assert!(
            projected
                .majors
                .iter()
                .any(|major| matches!(major.kind, AnchorMajorKind::HalfOrigin)
                    && major.current
                    && !major.supported),
            "the half origin marks the current placement (display-only)"
        );
    }

    /// A minimal Core-valid document for the preview-equals-commit tests:
    /// one board, one layout, a 1x2 main, a 1x1 thumb anchored to main R1C1,
    /// their member parts, and an anchored part u1.
    fn engine_doc() -> ProjectDoc {
        use boardstudio_core::model::*;
        let switch: PartDefinition = serde_json::from_value(serde_json::json!(
            {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]}
        ))
        .unwrap();
        let member = |matrix: &str, row: u32, column: u32| Part {
            id: format!("matrix/{matrix}/r{row}c{column}"),
            definition_id: "switch".into(),
            reference: format!("{matrix}-{row}-{column}"),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            keycap: None,
            outline: None,
            locked: None,
            properties: None,
            generator_parameters: None,
            anchor: None,
        };
        let mut doc = ProjectDoc::empty("pick-engine", "Pick engine");
        doc.definitions = vec![switch];
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![
                member("main", 0, 0).id.clone(),
                member("main", 0, 1).id.clone(),
                member("thumb", 0, 0).id.clone(),
                "u1".into(),
            ],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc.layouts.push(Layout {
            id: "left".into(),
            name: "Left half".into(),
            board_id: "board".into(),
            matrix_ids: vec!["main".into(), "thumb".into()],
            part_ids: vec!["u1".into()],
            mirror_link: None,
        });
        doc.matrices.push(Matrix {
            id: "main".into(),
            name: Some("Main".into()),
            rows: 1,
            columns: 2,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            origin: Vec2 { x: 0.0, y: 0.0 },
            definition_id: "switch".into(),
            part_ids: vec![
                member("main", 0, 0).id.clone(),
                member("main", 0, 1).id.clone(),
            ],
            board_id: Some("board".into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_layout: vec![],
            column_layout: vec![],
            anchor: None,
            cells: vec![],
        });
        doc.matrices.push(Matrix {
            id: "thumb".into(),
            name: Some("Thumb".into()),
            rows: 1,
            columns: 1,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            origin: Vec2 { x: 0.0, y: 0.0 },
            definition_id: "switch".into(),
            part_ids: vec![member("thumb", 0, 0).id.clone()],
            board_id: Some("board".into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_layout: vec![],
            column_layout: vec![],
            anchor: Some(MatrixAnchor {
                matrix_id: "main".into(),
                row: 0,
                column: 0,
                follow: Follow {
                    x: true,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            }),
            cells: vec![],
        });
        doc.parts.push(member("main", 0, 0));
        doc.parts.push(member("main", 0, 1));
        doc.parts.push(member("thumb", 0, 0));
        doc.parts.push(Part {
            id: "u1".into(),
            definition_id: "switch".into(),
            reference: "U1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            keycap: None,
            outline: None,
            locked: None,
            properties: None,
            generator_parameters: None,
            anchor: Some(PartAnchor {
                matrix_id: "main".into(),
                row: 0,
                column: 0,
                follow: Follow {
                    x: true,
                    y: true,
                    rotation: true,
                },
                spread: 5.0,
                stagger: 2.0,
                splay: 0.0,
                offset: Vec2::default(),
                rotation: 0.0,
            }),
        });
        doc
    }

    fn open_engine(
        doc: ProjectDoc,
    ) -> (
        boardstudio_core::CoreEngine,
        boardstudio_core::model::SceneDelta,
        ProjectDoc,
    ) {
        use boardstudio_core::CoreEngine;
        use boardstudio_core::model::{CoreReply, CoreRequest};
        let mut engine = CoreEngine::new();
        let (scene, opened) = match engine.handle(CoreRequest::Open {
            id: "open".into(),
            document: doc,
        }) {
            CoreReply::Scene {
                scene, document, ..
            } => (scene, *document),
            other => panic!("expected scene: {other:?}"),
        };
        (engine, scene, opened)
    }

    fn edit_engine(
        engine: &mut boardstudio_core::CoreEngine,
        doc: &ProjectDoc,
        operation: boardstudio_core::model::EditOperation,
    ) -> ProjectDoc {
        use boardstudio_core::model::*;
        match engine.handle(CoreRequest::Edit {
            id: "edit".into(),
            command: EditCommand {
                base_revision: doc.revision,
                transaction_id: "edit".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation,
            },
        }) {
            CoreReply::Scene { document, .. } => *document,
            other => panic!("expected scene: {other:?}"),
        }
    }

    #[test]
    fn part_preview_matches_the_post_commit_pose() {
        use boardstudio_core::evaluated_part_pose;
        let doc = engine_doc();
        let selection = TreeContext::Component {
            part_id: Some("u1".into()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        };
        let target = PickTarget {
            matrix_id: "main".into(),
            row: 0,
            column: 1,
        };
        let preview = preview_anchor_poses(&doc, &selection, &target);
        let commit = pick_commit(&doc, &selection, &target).expect("a live key commits");
        let (mut engine, _, opened) = open_engine(doc);
        let revision_before = opened.revision;
        let edited = edit_engine(&mut engine, &opened, commit.operation);
        assert_eq!(
            edited.revision,
            revision_before + 1,
            "the pick lands as one revision"
        );
        let after = evaluated_part_pose(
            &edited,
            edited
                .parts
                .iter()
                .find(|part| part.id == "u1")
                .expect("the part survives"),
        );
        assert_eq!(
            preview,
            vec![("u1".to_owned(), after)],
            "the amber preview is the post-commit pose"
        );
        let stored = edited
            .parts
            .iter()
            .find(|part| part.id == "u1")
            .expect("the part survives")
            .anchor
            .clone()
            .expect("still anchored");
        assert_eq!((stored.spread, stored.stagger), (5.0, 2.0));
    }

    #[test]
    fn matrix_preview_matches_the_post_commit_members() {
        use boardstudio_core::evaluated_part_pose;
        let doc = engine_doc();
        let selection = TreeContext::Matrix {
            matrix_id: "thumb".into(),
        };
        let target = PickTarget {
            matrix_id: "main".into(),
            row: 0,
            column: 1,
        };
        let preview = preview_anchor_poses(&doc, &selection, &target);
        let commit = pick_commit(&doc, &selection, &target).expect("a live key commits");
        let (mut engine, _, opened) = open_engine(doc);
        let edited = edit_engine(&mut engine, &opened, commit.operation);
        let mut expected: Vec<(String, boardstudio_core::model::Pose2)> = edited
            .parts
            .iter()
            .filter(|part| part.id.starts_with("matrix/thumb/"))
            .map(|part| (part.id.clone(), evaluated_part_pose(&edited, part)))
            .collect();
        expected.sort_by(|a, b| a.0.cmp(&b.0));
        let mut preview_sorted = preview;
        preview_sorted.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            preview_sorted, expected,
            "the amber preview is the post-commit member poses"
        );
        assert!(
            preview_sorted
                .iter()
                .any(|(_, pose)| pose.at.x != 0.0 || pose.at.y != 0.0),
            "the thumb moves to the new anchor"
        );
    }
}
