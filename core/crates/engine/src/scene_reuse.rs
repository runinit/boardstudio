use super::*;

#[derive(Clone)]
pub(super) struct ValidatedBoardScene {
    pub geometry: geometry::BoardSceneGeometry,
    pub findings: Vec<Finding>,
    pub markers: Vec<FindingMarker>,
}

impl ValidatedBoardScene {
    pub fn new(doc: &ProjectDoc, geometry: geometry::BoardSceneGeometry) -> Self {
        let (findings, markers) =
            outline_validation::validate(doc, &geometry.contours, &geometry.outlines);
        Self {
            geometry,
            findings,
            markers,
        }
    }
}

/// One settled Preview within the current accepted document. The settled
/// stagger route preserves the ordered parts, their non-pose attributes and
/// matrix membership. With unchanged outline sources and board ownership,
/// the resulting pose bits identify both geometry and fabrication checks.
pub(super) struct StaggerPreview {
    poses: Vec<[u64; 3]>,
    outline: geometry::OutlineProjection,
    board: ValidatedBoardScene,
}

impl StaggerPreview {
    pub fn new(
        draft: &ProjectDoc,
        outline: geometry::OutlineProjection,
        board: ValidatedBoardScene,
    ) -> Self {
        Self {
            poses: draft.parts.iter().map(pose_bits).collect(),
            outline,
            board,
        }
    }

    pub fn matches(&self, draft: &ProjectDoc) -> bool {
        self.poses
            .iter()
            .copied()
            .eq(draft.parts.iter().map(pose_bits))
    }

    pub fn projection(self) -> (geometry::OutlineProjection, Option<ValidatedBoardScene>) {
        (self.outline, Some(self.board))
    }

    pub fn clone_projection(&self) -> (geometry::OutlineProjection, Option<ValidatedBoardScene>) {
        (self.outline.clone(), Some(self.board.clone()))
    }
}

fn pose_bits(part: &Part) -> [u64; 3] {
    [
        part.pose.at.x.to_bits(),
        part.pose.at.y.to_bits(),
        part.pose.rotation.to_bits(),
    ]
}

pub(super) fn is_column_stagger(operation: &EditOperation) -> bool {
    matches!(
        operation,
        EditOperation::SetMatrixPlacement {
            change: MatrixPlacementChange::ColumnStagger { .. },
            ..
        }
    )
}

/// Compare the settled draft, after topology, constraints and spacing have run.
/// Project name is absent from the scene payload and its diagnostics.
pub(super) fn replacement_keeps_scene(accepted: &ProjectDoc, next: &mut ProjectDoc) -> bool {
    if accepted.name == next.name {
        return false;
    }
    let name = std::mem::replace(&mut next.name, accepted.name.clone());
    // The typed comparison rejects most replacements cheaply. The wire representation
    // also distinguishes equal floats with different encodings (such as signed zero).
    let unchanged = *next == *accepted
        && serde_json::to_vec(next)
            .ok()
            .zip(serde_json::to_vec(accepted).ok())
            .is_some_and(|(next, accepted)| next == accepted);
    next.name = name;
    unchanged
}

/// In an unlinked layout with no part anchors, outline features consume stored
/// part poses and attributes. Compare the settled parts, including any
/// changes from layout and constraints, independently of the edit receipt.
pub(super) fn settled_outline_changes(
    accepted: &ProjectDoc,
    next: &ProjectDoc,
    operation: &EditOperation,
) -> Option<Vec<String>> {
    if !(is_column_stagger(operation) || matches!(operation, EditOperation::MoveParts { .. }))
        || accepted.definitions != next.definitions
        || accepted.layouts != next.layouts
        || accepted
            .layouts
            .iter()
            .any(|layout| layout.mirror_link.is_some())
        || accepted.boards != next.boards
        || accepted.outline != next.outline
        || !next.board_outlines.is_empty()
        || accepted.parts.len() != next.parts.len()
    {
        return None;
    }
    let mut changed = Vec::new();
    for (before, after) in accepted.parts.iter().zip(&next.parts) {
        if before.id != after.id || before.anchor.is_some() || after.anchor.is_some() {
            return None;
        }
        if before != after || pose_bits(before) != pose_bits(after) {
            changed.push(after.id.clone());
        }
    }
    Some(changed)
}
