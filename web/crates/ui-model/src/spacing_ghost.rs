//! Ticket 18: ghost of the previous spacing.
//!
//! After a committed edit changes a split pair's evaluated spacing, Layout
//! draws a dashed ghost of the keys at the previous spacing with a short
//! "+N mm" / "−N mm" note on the axis, until the next edit or selection
//! change. This module holds the document comparison behind that ghost: which
//! pair moved, by how much, and how the note reads. Rendering and lifetime
//! live in the page shell; the math here stays pure so native tests pin it.

use crate::tree::ScopedTreeContext;
use boardstudio_core::model::{ProjectDoc, SceneDelta};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A spacing change between two accepted documents of the same board.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpacingChange {
    pub axis_x: f64,
    pub delta_mm: f64,
}

/// Changes at or below this (mm) never raise a ghost: the shift is invisible
/// and the note would read "+0.0 mm".
const GHOST_EPSILON_MM: f64 = 1e-6;

/// Effective spacing of one mirror-link layout: the evaluated `spacing`,
/// falling back to the stored gap exactly as Core's adaptive pass does.
fn effective_spacing(split_gap: Option<f64>, spacing: Option<f64>) -> Option<f64> {
    spacing.or(split_gap)
}

/// A layout's link before the commit, when it carried one.
#[derive(Clone, Copy, Debug)]
struct PreviousLink {
    split_gap: Option<f64>,
    spacing: Option<f64>,
}

/// The first split pair whose effective spacing moved between two accepted
/// documents, or `None` when nothing ghosts: different documents, no shared
/// pair, or every pair within epsilon. Both directions count; narrowing
/// reports a negative delta.
///
/// A newly adopted pair (gapless link before, stored gap plus spacing after,
/// as the first committed edit of a touching legacy board adopts it) ghosts
/// the widening above its freshly stored floor. A pair the commit created is
/// creation, not a spacing change, and never ghosts (ticket 11 owns the
/// creation gap).
pub fn spacing_change(previous: &ProjectDoc, next: &ProjectDoc) -> Option<SpacingChange> {
    if previous.id != next.id {
        return None;
    }
    // Previous link per layout id: `None` while the layout carries no link.
    let mut before: BTreeMap<&str, Option<PreviousLink>> = BTreeMap::new();
    for layout in &previous.layouts {
        before.insert(
            layout.id.as_str(),
            layout.mirror_link.as_ref().map(|link| PreviousLink {
                split_gap: link.split_gap,
                spacing: link.spacing,
            }),
        );
    }
    for layout in &next.layouts {
        let Some(link) = layout.mirror_link.as_ref() else {
            continue;
        };
        let new_effective = effective_spacing(link.split_gap, link.spacing);
        // A layout the commit created, or a bare layout it linked, has no
        // previous spacing to ghost (creation owns its gap). Only a layout
        // that already carried a link can adopt one this commit.
        let entry = before.get(layout.id.as_str());
        let had_link = matches!(entry, Some(Some(_)));
        let old_effective = match entry {
            Some(Some(old)) => effective_spacing(old.split_gap, old.spacing),
            _ => None,
        };
        if let (Some(old), Some(new)) = (old_effective, new_effective)
            && (new - old).abs() > GHOST_EPSILON_MM
        {
            return Some(SpacingChange {
                axis_x: link.axis_x,
                delta_mm: new - old,
            });
        }
        if old_effective.is_none()
            && had_link
            && let (Some(floor), Some(spacing)) = (link.split_gap, link.spacing)
            && spacing - floor > GHOST_EPSILON_MM
        {
            // Adopted by this commit: the stored floor preserves the old
            // pose, so the ghost spans spacing above it.
            return Some(SpacingChange {
                axis_x: link.axis_x,
                delta_mm: spacing - floor,
            });
        }
    }
    None
}

/// The last accepted snapshot the ghost tracker has processed.
#[derive(Clone, Debug, PartialEq)]
pub struct SeenAccepted {
    pub revision: u64,
    pub transaction: String,
    pub document: Arc<ProjectDoc>,
    pub scene: Arc<SceneDelta>,
}

/// One paint's accepted snapshot plus the live selection behind it.
#[derive(Clone, Debug, PartialEq)]
pub struct AcceptedView {
    pub revision: u64,
    pub transaction: String,
    pub document: Arc<ProjectDoc>,
    pub scene: Arc<SceneDelta>,
    pub selected_ids: Vec<String>,
    pub selected_context: Option<ScopedTreeContext>,
}

/// The live ghost: the pre-commit scene, the note, and the revision and
/// selection it was born with.
#[derive(Clone, Debug, PartialEq)]
pub struct SpacingGhostState {
    pub scene: Arc<SceneDelta>,
    pub delta_mm: f64,
    pub axis_x: f64,
    pub revision: u64,
    pub selected_ids: Vec<String>,
    pub selected_context: Option<ScopedTreeContext>,
}

/// Outcome of feeding one paint through the tracker. `Keep` leaves both
/// signals untouched so selection-only paints (previews, pans, hovers) never
/// disturb a settled ghost or cause render loops.
#[derive(Clone, Debug, PartialEq)]
pub enum GhostAdvance {
    Keep,
    Set {
        seen: SeenAccepted,
        ghost: Option<SpacingGhostState>,
    },
}

/// Core history steps mint a fresh revision but restore an old document; they
/// retire the ghost instead of raising a new one, matching the prototype.
fn is_history_step(transaction: &str) -> bool {
    transaction == "undo" || transaction == "redo"
}

/// Advance the tracker for one paint. A committed spacing change raises the
/// pre-commit scene as the ghost; the next edit, an undo/redo step, opening
/// another document, or any selection change retires it. A paint whose
/// document is byte-identical to the processed one (a no-op commit) keeps a
/// settled ghost: nothing edited, nothing to retire.
pub fn advance_spacing_ghost(
    seen: Option<&SeenAccepted>,
    current: &AcceptedView,
    ghost: Option<&SpacingGhostState>,
) -> GhostAdvance {
    let seen_matches = seen.is_some_and(|seen| {
        seen.revision == current.revision && seen.transaction == current.transaction
    });
    if seen_matches {
        let selection_moved = ghost.is_some_and(|ghost| {
            ghost.selected_ids != current.selected_ids
                || ghost.selected_context != current.selected_context
        });
        if selection_moved {
            return GhostAdvance::Set {
                seen: seen.expect("matched seen is present").clone(),
                ghost: None,
            };
        }
        return GhostAdvance::Keep;
    }
    let seen_now = SeenAccepted {
        revision: current.revision,
        transaction: current.transaction.clone(),
        document: current.document.clone(),
        scene: current.scene.clone(),
    };
    if let Some(seen) = seen
        && seen.document == current.document
    {
        // Same content under a new transaction: a no-op commit. The ghost it
        // settled (if any) still describes the previous spacing.
        return GhostAdvance::Keep;
    }
    if is_history_step(&current.transaction) {
        return GhostAdvance::Set {
            seen: seen_now,
            ghost: None,
        };
    }
    let change = seen
        .filter(|seen| seen.document.id == current.document.id)
        .and_then(|seen| spacing_change(&seen.document, &current.document));
    let ghost = change.and_then(|change| {
        let scene = seen
            .filter(|seen| seen.document.id == current.document.id)
            .map(|seen| seen.scene.clone())?;
        Some(SpacingGhostState {
            scene,
            delta_mm: change.delta_mm,
            axis_x: change.axis_x,
            revision: current.revision,
            selected_ids: current.selected_ids.clone(),
            selected_context: current.selected_context.clone(),
        })
    });
    GhostAdvance::Set {
        seen: seen_now,
        ghost,
    }
}

/// Short axis note for a spacing delta: "+5.0 mm", or "−5.0 mm" with U+2212
/// minus as the ticket writes it. One decimal, matching the prototype.
pub fn spacing_note(delta_mm: f64) -> String {
    let rounded = (delta_mm * 10.0).round() / 10.0;
    if rounded < 0.0 {
        format!("−{:.1} mm", rounded.abs())
    } else {
        format!("+{:.1} mm", rounded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{Layout, LayoutMirrorLink};

    fn link(axis_x: f64, split_gap: Option<f64>, spacing: Option<f64>) -> Layout {
        Layout {
            id: "paired".into(),
            name: "Right".into(),
            board_id: "board".into(),
            matrix_ids: vec![],
            part_ids: vec![],
            mirror_link: (split_gap.is_some() || spacing.is_some()).then(|| LayoutMirrorLink {
                correspondences: vec![],
                exceptions: vec![],
                source_id: "source".into(),
                axis_x,
                split_gap,
                spacing,
            }),
        }
    }

    fn bare_layout() -> Layout {
        Layout {
            id: "paired".into(),
            name: "Right".into(),
            board_id: "board".into(),
            matrix_ids: vec![],
            part_ids: vec![],
            mirror_link: None,
        }
    }

    fn gapless_link() -> Layout {
        let mut layout = bare_layout();
        layout.mirror_link = Some(LayoutMirrorLink {
            correspondences: vec![],
            exceptions: vec![],
            source_id: "source".into(),
            axis_x: 0.0,
            split_gap: None,
            spacing: None,
        });
        layout
    }

    fn document(id: &str, layouts: Vec<Layout>) -> ProjectDoc {
        let mut document = ProjectDoc::empty(id, "Ghost");
        document.layouts = layouts;
        document
    }

    #[test]
    fn widening_reports_a_positive_delta_on_the_pair_axis() {
        let change = spacing_change(
            &document("board", vec![link(0.0, Some(40.0), Some(40.0))]),
            &document("board", vec![link(0.0, Some(40.0), Some(50.1))]),
        )
        .expect("widening must ghost");
        assert_eq!(change.axis_x, 0.0);
        assert!((change.delta_mm - 10.1).abs() < 1e-9);
        assert_eq!(spacing_note(change.delta_mm), "+10.1 mm");
    }

    #[test]
    fn narrowing_reports_a_negative_delta_with_prototype_minus() {
        let change = spacing_change(
            &document("board", vec![link(2.5, Some(40.0), Some(50.0))]),
            &document("board", vec![link(2.5, Some(40.0), Some(44.0))]),
        )
        .expect("narrowing must ghost");
        assert_eq!(change.axis_x, 2.5);
        assert!((change.delta_mm + 6.0).abs() < 1e-9);
        assert_eq!(spacing_note(change.delta_mm), "−6.0 mm");
    }

    #[test]
    fn unchanged_spacing_and_new_pairs_never_ghost() {
        assert_eq!(
            spacing_change(
                &document("board", vec![link(0.0, Some(40.0), Some(40.0))]),
                &document("board", vec![link(0.0, Some(40.0), Some(40.0))]),
            ),
            None
        );
        assert_eq!(
            spacing_change(
                &document("board", vec![]),
                &document("board", vec![link(0.0, Some(40.0), Some(50.0))]),
            ),
            None,
            "a pair created by the commit itself is creation, not a spacing change"
        );
        assert_eq!(
            spacing_change(
                &document("board", vec![bare_layout()]),
                &document("board", vec![link(0.0, Some(40.0), Some(50.0))]),
            ),
            None,
            "linking a bare layout has no previous spacing to ghost"
        );
        assert_eq!(
            spacing_change(
                &document("board", vec![link(0.0, Some(40.0), Some(40.0))]),
                &document("other", vec![link(0.0, Some(40.0), Some(50.0))]),
            ),
            None,
            "opening another document is not a spacing edit"
        );
    }

    #[test]
    fn float_dust_below_a_micron_never_ghost() {
        assert_eq!(
            spacing_change(
                &document("board", vec![link(0.0, Some(40.0), Some(40.0))]),
                &document("board", vec![link(0.0, Some(40.0), Some(40.0 + 1e-9))]),
            ),
            None
        );
    }

    #[test]
    fn adopted_legacy_pair_ghosts_the_widening_above_its_floor() {
        let change = spacing_change(
            &document("board", vec![gapless_link()]),
            &document("board", vec![link(0.0, Some(40.0), Some(47.5))]),
        )
        .expect("first-edit widening of a touching board must ghost");
        assert!((change.delta_mm - 7.5).abs() < 1e-9);
        assert_eq!(spacing_note(change.delta_mm), "+7.5 mm");
    }

    fn scene(transaction: &str) -> Arc<SceneDelta> {
        Arc::new(
            serde_json::from_value(serde_json::json!({
                "revision": 0, "transactionId": transaction, "changedIds": [], "transforms": [],
                "matrixScenes": [], "contours": [], "boardContours": [], "boardReadiness": [],
                "findings": [], "readiness": {"layout": true, "outline": false, "pcb": false, "case": false}
            }))
            .unwrap(),
        )
    }

    fn view(
        document: ProjectDoc,
        revision: u64,
        transaction: &str,
        selected: &[&str],
    ) -> AcceptedView {
        AcceptedView {
            revision,
            transaction: transaction.into(),
            document: Arc::new(document),
            scene: scene(transaction),
            selected_ids: selected.iter().map(|id| id.to_string()).collect(),
            selected_context: None,
        }
    }

    fn advance(
        seen: Option<&SeenAccepted>,
        current: &AcceptedView,
        ghost: Option<&SpacingGhostState>,
    ) -> GhostAdvance {
        advance_spacing_ghost(seen, current, ghost)
    }

    fn seen_of(current: &AcceptedView) -> SeenAccepted {
        SeenAccepted {
            revision: current.revision,
            transaction: current.transaction.clone(),
            document: current.document.clone(),
            scene: current.scene.clone(),
        }
    }

    #[test]
    fn first_paint_records_seen_without_ghost() {
        let current = view(
            document("board", vec![link(0.0, Some(40.0), Some(40.0))]),
            1,
            "open",
            &[],
        );
        assert_eq!(
            advance(None, &current, None),
            GhostAdvance::Set {
                seen: seen_of(&current),
                ghost: None,
            }
        );
    }

    #[test]
    fn widening_commit_raises_the_pre_commit_scene_as_ghost() {
        let before = view(
            document("board", vec![link(0.0, Some(40.0), Some(40.0))]),
            1,
            "open",
            &[],
        );
        let after = view(
            document("board", vec![link(0.0, Some(40.0), Some(50.0))]),
            2,
            "edit-1",
            &["matrix/left/r0c0"],
        );
        let GhostAdvance::Set { seen, ghost } = advance(Some(&seen_of(&before)), &after, None)
        else {
            panic!("a widening commit must settle the tracker")
        };
        assert_eq!(seen.revision, 2);
        let ghost = ghost.expect("widening must ghost");
        assert!(Arc::ptr_eq(&ghost.scene, &before.scene));
        assert!((ghost.delta_mm - 10.0).abs() < 1e-9);
        assert_eq!(ghost.axis_x, 0.0);
        assert_eq!(ghost.revision, 2);
        assert_eq!(ghost.selected_ids, vec!["matrix/left/r0c0".to_owned()]);
    }

    #[test]
    fn narrowing_commit_reports_a_negative_delta() {
        let before = view(
            document("board", vec![link(0.0, Some(40.0), Some(50.0))]),
            2,
            "edit-1",
            &[],
        );
        let after = view(
            document("board", vec![link(0.0, Some(40.0), Some(44.0))]),
            3,
            "edit-2",
            &[],
        );
        let GhostAdvance::Set { ghost, .. } = advance(Some(&seen_of(&before)), &after, None) else {
            panic!("a narrowing commit must settle the tracker")
        };
        let ghost = ghost.expect("narrowing must ghost");
        assert!((ghost.delta_mm + 6.0).abs() < 1e-9);
    }

    #[test]
    fn next_edit_without_spacing_change_retires_the_ghost() {
        let before = view(
            document("board", vec![link(0.0, Some(40.0), Some(50.0))]),
            2,
            "edit-1",
            &[],
        );
        let seen = seen_of(&before);
        let ghost = SpacingGhostState {
            scene: before.scene.clone(),
            delta_mm: 10.0,
            axis_x: 0.0,
            revision: 2,
            selected_ids: vec![],
            selected_context: None,
        };
        // Same spacing, moved matrix origin: the document differs, the ghost ends.
        let mut moved = (*before.document).clone();
        moved.matrices.push(
            serde_json::from_value(serde_json::json!({
                "id": "m", "rows": 1, "columns": 1,
                "pitch": {"x": 19.0, "y": 19.0}, "origin": {"x": 1.0, "y": 0.0},
                "definitionId": "switch", "partIds": [], "boardId": "board",
                "cells": [{"row": 0, "column": 0, "enabled": true, "assemblies": []}]
            }))
            .unwrap(),
        );
        let after = view(moved, 3, "edit-2", &[]);
        assert_eq!(
            advance(Some(&seen), &after, Some(&ghost)),
            GhostAdvance::Set {
                seen: seen_of(&after),
                ghost: None,
            }
        );
    }

    #[test]
    fn no_op_commit_keeps_a_settled_ghost() {
        let before = view(
            document("board", vec![link(0.0, Some(40.0), Some(50.0))]),
            2,
            "edit-1",
            &[],
        );
        let seen = seen_of(&before);
        let ghost = SpacingGhostState {
            scene: before.scene.clone(),
            delta_mm: 10.0,
            axis_x: 0.0,
            revision: 2,
            selected_ids: vec![],
            selected_context: None,
        };
        // Same content under a fresh transaction: nothing edited, nothing retires.
        let after = view((*before.document).clone(), 2, "edit-2", &[]);
        assert_eq!(
            advance(Some(&seen), &after, Some(&ghost)),
            GhostAdvance::Keep
        );
    }

    #[test]
    fn undo_and_redo_retire_the_ghost_without_raising_a_new_one() {
        let before = view(
            document("board", vec![link(0.0, Some(40.0), Some(50.0))]),
            2,
            "edit-1",
            &[],
        );
        let seen = seen_of(&before);
        let ghost = SpacingGhostState {
            scene: before.scene.clone(),
            delta_mm: 10.0,
            axis_x: 0.0,
            revision: 2,
            selected_ids: vec![],
            selected_context: None,
        };
        for transaction in ["undo", "redo"] {
            let restored = view(
                document("board", vec![link(0.0, Some(40.0), Some(40.0))]),
                3,
                transaction,
                &[],
            );
            assert_eq!(
                advance(Some(&seen), &restored, Some(&ghost)),
                GhostAdvance::Set {
                    seen: seen_of(&restored),
                    ghost: None,
                },
                "{transaction} must retire the ghost, not re-raise it"
            );
        }
    }

    #[test]
    fn selection_change_retires_the_ghost_while_still_paints_keep_it() {
        let before = view(
            document("board", vec![link(0.0, Some(40.0), Some(40.0))]),
            1,
            "open",
            &[],
        );
        let widened = view(
            document("board", vec![link(0.0, Some(40.0), Some(50.0))]),
            2,
            "edit-1",
            &["matrix/left/r0c0"],
        );
        let seen = seen_of(&before);
        let GhostAdvance::Set { ghost, .. } = advance(Some(&seen), &widened, None) else {
            panic!("widening must ghost")
        };
        let ghost = ghost.expect("widening must ghost");
        let seen_widened = seen_of(&widened);
        // Same accepted snapshot, same selection: a still paint keeps the ghost.
        assert_eq!(
            advance(Some(&seen_widened), &widened, Some(&ghost)),
            GhostAdvance::Keep
        );
        // Same accepted snapshot, new part selection: the ghost retires.
        let reselected = view(
            (*widened.document).clone(),
            2,
            "edit-1",
            &["matrix/right/r0c0"],
        );
        assert_eq!(
            advance(Some(&seen_widened), &reselected, Some(&ghost)),
            GhostAdvance::Set {
                seen: seen_widened.clone(),
                ghost: None,
            }
        );
        // Same accepted snapshot, new tree context: the ghost retires too.
        let mut tree_selected = widened.clone();
        tree_selected.selected_context = Some(ScopedTreeContext {
            scope: boardstudio_application::Scope {
                session_epoch: boardstudio_application::SessionEpoch(1),
                document_id: "board".into(),
                board_id: "board".into(),
                instance_id: None,
            },
            context: crate::tree::TreeContext::Matrix {
                matrix_id: "left".into(),
            },
        });
        assert_eq!(
            advance(Some(&seen_widened), &tree_selected, Some(&ghost)),
            GhostAdvance::Set {
                seen: seen_widened,
                ghost: None,
            }
        );
    }

    #[test]
    fn opening_another_document_retires_the_ghost() {
        let before = view(
            document("board", vec![link(0.0, Some(40.0), Some(50.0))]),
            2,
            "edit-1",
            &[],
        );
        let seen = seen_of(&before);
        let ghost = SpacingGhostState {
            scene: before.scene.clone(),
            delta_mm: 10.0,
            axis_x: 0.0,
            revision: 2,
            selected_ids: vec![],
            selected_context: None,
        };
        let opened = view(
            document("other", vec![link(0.0, Some(40.0), Some(60.0))]),
            1,
            "open",
            &[],
        );
        assert_eq!(
            advance(Some(&seen), &opened, Some(&ghost)),
            GhostAdvance::Set {
                seen: seen_of(&opened),
                ghost: None,
            }
        );
    }

    #[test]
    fn adopted_legacy_pair_sets_the_ghost_through_the_tracker() {
        let before = view(document("board", vec![gapless_link()]), 1, "open", &[]);
        let after = view(
            document("board", vec![link(0.0, Some(40.0), Some(47.5))]),
            2,
            "edit-1",
            &[],
        );
        let GhostAdvance::Set { ghost, .. } = advance(Some(&seen_of(&before)), &after, None) else {
            panic!("adoption widening must ghost")
        };
        let ghost = ghost.expect("adoption widening must ghost");
        assert!((ghost.delta_mm - 7.5).abs() < 1e-9);
    }

    #[test]
    fn adopted_pair_kept_on_its_floor_never_ghost() {
        assert_eq!(
            spacing_change(
                &document("board", vec![gapless_link()]),
                &document("board", vec![link(0.0, Some(40.0), Some(40.0))]),
            ),
            None
        );
        assert_eq!(
            spacing_change(
                &document("board", vec![gapless_link()]),
                &document("board", vec![link(0.0, Some(40.0), None)]),
            ),
            None,
            "adopted but unspaced: the pose did not move"
        );
    }
}
