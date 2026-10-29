//! Ticket 17: the canvas anchor badge for the selection.
//!
//! A pure projection of the accepted document and Core's evaluated matrix
//! scenes into the badge the Layout canvas draws at what the selection is
//! placed from: the anchor point, the selection base the dashed tie runs to,
//! the member part IDs that keep the violet dashed target, and the short
//! badge label. Ticket 13 covers independent parts too; matrix, column, row
//! and key selections work on either half, dimmed or not.

use boardstudio_core::model::{Matrix, MatrixScene, MatrixSceneCell, ProjectDoc, Vec2};
use boardstudio_web_ui_model::tree::TreeContext;

use super::matrix_relations::{aggregate_kind_text, follow_text};
use boardstudio_core::{evaluated_part_pose, half_offset, layout_half_offset};
use boardstudio_web_ui_model::split_focus::{SplitHalf, part_half};

/// What the selection is placed from, in world millimetres.
#[derive(Clone, Debug, PartialEq)]
pub struct AnchorBadge {
    /// World point of the anchor badge itself.
    pub anchor: Vec2,
    /// World point of the selection base the dashed tie runs to.
    pub selection: Vec2,
    /// Member part IDs that keep the violet dashed target highlight.
    pub highlight: Vec<String>,
    /// Short badge label, for example `Column 2 · spread, stagger, splay carry`.
    pub label: String,
    /// Whether to draw the dashed tie. Unset when the badge sits on the
    /// selection itself (first row, first-row key) or the points coincide.
    pub tied: bool,
}

/// Lookup context for one matrix selection: the document, Core's evaluated
/// scenes, and the selected matrix. One value travels instead of three.
struct Cx<'a> {
    document: &'a ProjectDoc,
    scenes: &'a [&'a MatrixScene],
    matrix: &'a Matrix,
}

impl<'a> Cx<'a> {
    fn new(
        document: &'a ProjectDoc,
        scenes: &'a [&'a MatrixScene],
        matrix_id: &str,
    ) -> Option<Self> {
        document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| Self {
                document,
                scenes,
                matrix,
            })
    }

    fn scene_of(scenes: &[&'a MatrixScene], matrix_id: &str) -> Option<&'a MatrixScene> {
        scenes
            .iter()
            .copied()
            .find(|scene| scene.matrix_id == matrix_id)
    }

    fn scene(&self) -> Option<&'a MatrixScene> {
        Self::scene_of(self.scenes, &self.matrix.id)
    }

    /// The single evaluated-cell lookup every pose and member read shares.
    fn cell(&self, matrix_id: &str, row: u32, column: u32) -> Option<&MatrixSceneCell> {
        Self::scene_of(self.scenes, matrix_id).and_then(|scene| {
            scene
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
        })
    }

    fn own_cell(&self, row: u32, column: u32) -> Option<&MatrixSceneCell> {
        let id = self.matrix.id.clone();
        self.cell(&id, row, column)
    }

    fn pose(&self, row: u32, column: u32) -> Option<Vec2> {
        self.own_cell(row, column).map(|cell| cell.pose.at)
    }

    fn own_member(&self, row: u32, column: u32) -> Option<String> {
        self.own_cell(row, column)
            .and_then(|cell| cell.member_id.clone())
    }

    fn member(&self, matrix_id: &str, row: u32, column: u32) -> Option<String> {
        self.cell(matrix_id, row, column)
            .and_then(|cell| cell.member_id.clone())
    }

    fn members(&self, keep: impl Fn(u32, u32) -> bool) -> Vec<String> {
        let Some(scene) = self.scene() else {
            return Vec::new();
        };
        (0..self.matrix.rows)
            .flat_map(|row| (0..self.matrix.columns).map(move |column| (row, column)))
            .filter(|(row, column)| keep(*row, *column))
            .filter_map(|(row, column)| {
                scene
                    .cells
                    .iter()
                    .find(|cell| cell.row == row && cell.column == column)
                    .and_then(|cell| cell.member_id.clone())
            })
            .collect()
    }

    /// A column basis base in world millimetres: what Core places that column
    /// from. Column 0 is the evaluated matrix frame origin.
    fn column_base(&self, column: u32) -> Option<Vec2> {
        self.scene().and_then(|scene| {
            scene
                .columns
                .iter()
                .find(|basis| basis.column == column)
                .map(|basis| basis.base)
        })
    }

    fn frame_origin(&self) -> Option<Vec2> {
        self.column_base(0)
    }

    fn display_name(&self, matrix_id: &str) -> String {
        self.document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .and_then(|matrix| matrix.name.clone())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| matrix_id.to_owned())
    }

    /// The matrix anchor as (point, highlight, label): the anchor key, or the
    /// mean of aggregate parts. `None` when unanchored or unresolvable (a
    /// removed anchor key leaves no badge, like the inert Follows row).
    /// Shared by the matrix badge and the first-column badge, so the origin
    /// suffix names the same anchor the frame follows (prototype variant B).
    fn anchor_description(&self) -> Option<(Vec2, Vec<String>, String)> {
        let anchor = self.matrix.anchor.as_ref()?;
        if let Some(aggregate) = anchor.aggregate.as_ref() {
            // An aggregate anchor derives one frame from its parts: the badge
            // sits at their mean pose and highlights every part key.
            let poses: Vec<Vec2> = aggregate
                .parts
                .iter()
                .filter_map(|part| {
                    self.cell(&part.matrix_id, part.row, part.column)
                        .map(|cell| cell.pose.at)
                })
                .collect();
            if poses.is_empty() {
                return None;
            }
            let count = poses.len() as f64;
            let point = Vec2 {
                x: poses.iter().map(|pose| pose.x).sum::<f64>() / count,
                y: poses.iter().map(|pose| pose.y).sum::<f64>() / count,
            };
            let highlight: Vec<String> = aggregate
                .parts
                .iter()
                .filter_map(|part| self.member(&part.matrix_id, part.row, part.column))
                .collect();
            let kind = aggregate_kind_text(anchor).unwrap_or_else(|| "Anchor".to_owned());
            return Some((
                point,
                highlight,
                format!("{} · follows {}", kind, follow_text(&anchor.follow)),
            ));
        }
        let point = self
            .cell(&anchor.matrix_id, anchor.row, anchor.column)
            .map(|cell| cell.pose.at)?;
        let highlight: Vec<String> = self
            .member(&anchor.matrix_id, anchor.row, anchor.column)
            .into_iter()
            .collect();
        Some((
            point,
            highlight,
            format!(
                "{} R{}C{} · follows {}",
                self.display_name(&anchor.matrix_id),
                anchor.row + 1,
                anchor.column + 1,
                follow_text(&anchor.follow)
            ),
        ))
    }
}

/// Whether the dashed tie reads: unset when the badge sits on the selection
/// itself or the points coincide (prototype variant B uses 0.5 mm).
fn is_tied(anchor: Vec2, selection: Vec2) -> bool {
    (anchor.x - selection.x).hypot(anchor.y - selection.y) > 0.5
}

/// Whether a part rides the matrix refresh (a member or generated companion)
/// instead of the part anchor vocabulary: its key or column badge covers it.
fn is_matrix_part(scenes: &[&MatrixScene], part_id: &str) -> bool {
    scenes.iter().any(|scene| {
        scene
            .cells
            .iter()
            .any(|cell| cell.member_id.as_deref() == Some(part_id))
            || scene
                .companions
                .iter()
                .any(|companion| companion.part_id == part_id)
    })
}

/// The anchor badge for an independent part (ticket 13): what the part is
/// placed from. Anchored parts badge their anchor key like an anchored
/// matrix; detached parts badge their half origin like an unanchored matrix.
/// `None` for matrix members and companions (their key badge covers them),
/// for a missing part, or when the anchor key has no evaluated pose.
fn part_badge(
    document: &ProjectDoc,
    scenes: &[&MatrixScene],
    part_id: &str,
) -> Option<AnchorBadge> {
    let part = document.parts.iter().find(|part| part.id == part_id)?;
    if is_matrix_part(scenes, part_id) {
        return None;
    }
    let selection = evaluated_part_pose(document, part).at;
    let Some(anchor) = part.anchor.as_ref() else {
        // Detached parts hang off their half origin (the default), like
        // unanchored matrices; ungrouped parts badge their own base.
        let half = part_half(document, part_id);
        let origin = half
            .and_then(|_| {
                document
                    .layouts
                    .iter()
                    .find(|layout| layout.part_ids.iter().any(|id| id == part_id))
                    .and_then(|layout| layout_half_offset(document, &layout.id))
            })
            .unwrap_or(selection);
        let label = half.map_or_else(
            || format!("{} origin · default", part.reference),
            |half| {
                format!(
                    "{} half origin · default",
                    if half == SplitHalf::Left {
                        "Left"
                    } else {
                        "Right"
                    }
                )
            },
        );
        return Some(AnchorBadge {
            tied: is_tied(origin, selection),
            anchor: origin,
            selection,
            highlight: Vec::new(),
            label,
        });
    };
    let cx = Cx::new(document, scenes, &anchor.matrix_id)?;
    let point = cx
        .cell(&anchor.matrix_id, anchor.row, anchor.column)
        .map(|cell| cell.pose.at)?;
    let highlight: Vec<String> = cx
        .member(&anchor.matrix_id, anchor.row, anchor.column)
        .into_iter()
        .collect();
    Some(AnchorBadge {
        tied: is_tied(point, selection),
        anchor: point,
        selection,
        highlight,
        label: format!(
            "{} R{}C{} · follows {}",
            cx.display_name(&anchor.matrix_id),
            anchor.row + 1,
            anchor.column + 1,
            follow_text(&anchor.follow)
        ),
    })
}

/// The anchor badge for a Layout selection, or `None` when the selection is
/// cleared, names no matrix object, or its anchor has no evaluated pose.
pub fn anchor_badge(
    document: &ProjectDoc,
    scenes: &[&MatrixScene],
    context: Option<&TreeContext>,
) -> Option<AnchorBadge> {
    match context? {
        TreeContext::Matrix { matrix_id } => matrix_badge(document, scenes, matrix_id),
        TreeContext::Column { matrix_id, column } => {
            column_badge(document, scenes, matrix_id, *column)
        }
        TreeContext::Row { matrix_id, row } => row_badge(document, scenes, matrix_id, *row),
        TreeContext::Key {
            matrix_id,
            row,
            column,
        } => key_badge(document, scenes, matrix_id, *row, *column),
        TreeContext::Component {
            part_id: Some(part_id),
            matrix_id: None,
            ..
        } => part_badge(document, scenes, part_id),
        TreeContext::Component {
            matrix_id: Some(matrix_id),
            row: Some(row),
            column: Some(column),
            ..
        } => key_badge(document, scenes, matrix_id, *row, *column),
        _ => None,
    }
}

fn matrix_badge(
    document: &ProjectDoc,
    scenes: &[&MatrixScene],
    matrix_id: &str,
) -> Option<AnchorBadge> {
    let cx = Cx::new(document, scenes, matrix_id)?;
    let selection = cx.frame_origin()?;
    if cx.matrix.anchor.is_some() {
        // An anchored matrix with no evaluated anchor pose (a removed anchor
        // key) leaves no badge, like the inert Follows row.
        let (point, highlight, label) = cx.anchor_description()?;
        return Some(AnchorBadge {
            tied: is_tied(point, selection),
            anchor: point,
            selection,
            highlight,
            label,
        });
    }
    // Unanchored matrices hang off their half origin (the default), or
    // the matrix origin where no half frame exists.
    let half = boardstudio_web_ui_model::split_focus::matrix_half(document, matrix_id);
    let origin = half
        .and_then(|_| half_offset(document, matrix_id))
        .unwrap_or(selection);
    let label = half.map_or_else(
        || format!("{} origin · default", cx.display_name(matrix_id)),
        |half| {
            format!(
                "{} half origin · default",
                if half == boardstudio_web_ui_model::split_focus::SplitHalf::Left {
                    "Left"
                } else {
                    "Right"
                }
            )
        },
    );
    Some(AnchorBadge {
        tied: is_tied(origin, selection),
        anchor: origin,
        selection,
        highlight: Vec::new(),
        label,
    })
}

fn column_badge(
    document: &ProjectDoc,
    scenes: &[&MatrixScene],
    matrix_id: &str,
    column: u32,
) -> Option<AnchorBadge> {
    let cx = Cx::new(document, scenes, matrix_id)?;
    if column >= cx.matrix.columns {
        return None;
    }
    if column == 0 {
        // The first column hangs off the matrix origin, like the frame. An
        // anchored frame names what it follows (prototype variant B).
        let origin = cx.frame_origin()?;
        let selection = cx.pose(0, 0).unwrap_or(origin);
        let (highlight, suffix) = cx
            .anchor_description()
            .map(|(_, highlight, label)| (highlight, format!(" (← {label})")))
            .unwrap_or_default();
        return Some(AnchorBadge {
            tied: is_tied(origin, selection),
            anchor: origin,
            selection,
            highlight,
            label: format!("{} origin{suffix}", cx.display_name(matrix_id)),
        });
    }
    // Later columns carry spread, stagger and splay from the previous column.
    let point = cx
        .pose(0, column - 1)
        .or_else(|| cx.column_base(column - 1))?;
    let selection = cx.pose(0, column).or_else(|| cx.column_base(column))?;
    Some(AnchorBadge {
        tied: is_tied(point, selection),
        anchor: point,
        selection,
        highlight: cx.members(|_, c| c == column - 1),
        label: format!("Column {column} · spread, stagger, splay carry"),
    })
}

fn row_badge(
    document: &ProjectDoc,
    scenes: &[&MatrixScene],
    matrix_id: &str,
    row: u32,
) -> Option<AnchorBadge> {
    let cx = Cx::new(document, scenes, matrix_id)?;
    if row >= cx.matrix.rows {
        return None;
    }
    if row == 0 {
        // The first row sits on the column bases: the badge marks its own base.
        let point = cx.pose(0, 0).or_else(|| cx.frame_origin())?;
        return Some(AnchorBadge {
            tied: false,
            anchor: point,
            selection: point,
            highlight: Vec::new(),
            label: "Column bases".to_owned(),
        });
    }
    // Later rows carry padding, shift and bend from the row below.
    let point = cx.pose(row - 1, 0)?;
    let selection = cx.pose(row, 0)?;
    Some(AnchorBadge {
        tied: is_tied(point, selection),
        anchor: point,
        selection,
        highlight: cx.members(|r, _| r == row - 1),
        label: format!("Row {row} · padding, shift, bend carry"),
    })
}

fn key_badge(
    document: &ProjectDoc,
    scenes: &[&MatrixScene],
    matrix_id: &str,
    row: u32,
    column: u32,
) -> Option<AnchorBadge> {
    let cx = Cx::new(document, scenes, matrix_id)?;
    if row >= cx.matrix.rows || column >= cx.matrix.columns {
        return None;
    }
    let selection = cx.pose(row, column)?;
    if row == 0 {
        // A first-row key sits on its column base: the badge marks the key itself.
        return Some(AnchorBadge {
            tied: false,
            anchor: selection,
            selection,
            highlight: Vec::new(),
            label: format!("Column {} base", column + 1),
        });
    }
    // Carried keys hang off the key below them.
    let point = cx.pose(row - 1, column)?;
    Some(AnchorBadge {
        tied: is_tied(point, selection),
        anchor: point,
        selection,
        highlight: cx.own_member(row - 1, column).into_iter().collect(),
        label: format!("Key R{row}C{} · carried from below", column + 1),
    })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
