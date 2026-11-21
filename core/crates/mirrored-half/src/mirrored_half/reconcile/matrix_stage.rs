//! Staging of mirrored matrices and forward projection of matrices and parts.

use super::{
    Projection, matrix_members_for_pair, membership_view::fixed_membership_view,
    omitted_mirror_matrix, remove_omitted_matrix_parts,
};
use crate::model::{Matrix, MirroredCorrespondence, MirroredItemKind, Part, Pose2, ProjectDoc};
use std::collections::BTreeSet;

pub(super) struct RetainedCell {
    pub(super) member_ids: Vec<String>,
    pub(super) cell: crate::model::MatrixCell,
}

pub(super) struct MatrixStage {
    // Reconciliation reads pre-projection parts, nets, and ownership from the
    // baseline while the destructive matrix setter and later projectors update this draft.
    pub(super) draft: ProjectDoc,
    pub(super) source_members: BTreeSet<String>,
    pub(super) mirror_members: BTreeSet<String>,
    pub(super) omitted_parts: BTreeSet<String>,
    pub(super) allow_absent: bool,
}

pub(super) fn matrix_by_id<'a>(document: &'a ProjectDoc, id: &str) -> Result<&'a Matrix, String> {
    let mut matches = document.matrices.iter().filter(|matrix| matrix.id == id);
    let matrix = matches
        .next()
        .ok_or_else(|| format!("The mirrored matrix {id:?} is unavailable"))?;
    if matches.next().is_some() {
        return Err(format!("The mirrored matrix {id:?} is ambiguous"));
    }
    Ok(matrix)
}

pub(super) fn stage_matrix(
    document: &ProjectDoc,
    halves: &crate::model::MirroredHalves,
    projection: &Projection,
    mapping: Pose2,
    explicit: bool,
) -> Result<MatrixStage, String> {
    let Some(matrix_link) = projection.matrix.as_ref() else {
        if !projection.parts.is_empty() || !projection.locally_protected_mirror_parts.is_empty() {
            return Err("The matrixless projection retains Matrix ownership".into());
        }
        let (source_members, mirror_members) = matrix_members_for_pair(document, halves);
        return Ok(MatrixStage {
            draft: document.clone(),
            source_members,
            mirror_members,
            omitted_parts: BTreeSet::new(),
            allow_absent: true,
        });
    };
    let source_matrix_id = matrix_link.source_matrix_id.as_str();
    let mirror_matrix_id = matrix_link.mirror_matrix_id.as_str();
    let retired = projection.preserve_matrix_geometry
        && !document
            .matrices
            .iter()
            .any(|matrix| matrix.id == source_matrix_id)
        && !halves.correspondences.iter().any(|correspondence| {
            correspondence.kind == MirroredItemKind::Matrix
                && (correspondence.source_id == source_matrix_id
                    || correspondence.mirror_id == mirror_matrix_id)
        })
        && !halves.exceptions.iter().any(|exception| {
            exception.kind == MirroredItemKind::Matrix && exception.mirror_id == mirror_matrix_id
        });
    if retired {
        if !projection.parts.is_empty() || !projection.locally_protected_mirror_parts.is_empty() {
            return Err("The retired Matrix projection retains active ownership".into());
        }
        let (source_members, mirror_members) = matrix_members_for_pair(document, halves);
        return Ok(MatrixStage {
            draft: document.clone(),
            source_members,
            mirror_members,
            omitted_parts: BTreeSet::new(),
            allow_absent: true,
        });
    }
    let mut matrix_correspondences = halves.correspondences.iter().filter(|correspondence| {
        correspondence.kind == MirroredItemKind::Matrix
            && (correspondence.source_id == source_matrix_id
                || correspondence.mirror_id == mirror_matrix_id
                || correspondence.source_id == mirror_matrix_id
                || correspondence.mirror_id == source_matrix_id)
    });
    // A Local or Omitted matrix keeps its correspondence; the overlay is only its state.
    let exact_matrix_exception = halves.exceptions.iter().find(|exception| {
        exception.kind == MirroredItemKind::Matrix
            && exception.mirror_id == mirror_matrix_id
            && halves.overlay_source(exception) == Some(source_matrix_id)
    });
    let matrix_correspondence = matrix_correspondences
        .next()
        .ok_or_else(|| "The matrix correspondence is unavailable".to_owned())?;
    if matrix_correspondences.next().is_some()
        || matrix_correspondence.source_id != source_matrix_id
        || matrix_correspondence.mirror_id != mirror_matrix_id
    {
        return Err("The matrix correspondence is ambiguous or does not match the input".into());
    }

    let source_matrix = matrix_by_id(document, source_matrix_id)?;
    let absent_matrix;
    let mirror_matrix = match matrix_by_id(document, mirror_matrix_id) {
        Ok(matrix) => matrix,
        Err(_)
            if exact_matrix_exception.is_some_and(|exception| {
                exception.state == crate::model::MirroredExceptionState::Omitted
            }) =>
        {
            absent_matrix = omitted_mirror_matrix(
                source_matrix,
                mirror_matrix_id,
                &projection.mirror_board_id,
                halves,
            );
            &absent_matrix
        }
        Err(error) => return Err(error),
    };
    if source_matrix.board_id.as_deref() != Some(halves.source_board_id.as_str())
        || mirror_matrix.board_id.as_deref() != Some(halves.mirror_board_id.as_str())
    {
        return Err(
            "The matrix correspondence does not belong to its source and mirror boards".into(),
        );
    }

    crate::matrix::valid_matrix(source_matrix, document)?;
    crate::matrix::valid_matrix(mirror_matrix, document)?;
    if !projection.mapping.at.x.is_finite()
        || !projection.mapping.at.y.is_finite()
        || !projection.mapping.rotation.is_finite()
    {
        return Err("The reconciliation mapping contains non-finite geometry".into());
    }

    let view = if exact_matrix_exception.is_none() && !projection.preserve_matrix_geometry {
        let projected_matrix =
            project_matrix(source_matrix, mirror_matrix, matrix_correspondence, mapping)?;
        crate::matrix::valid_matrix(&projected_matrix, document)?;
        Some(fixed_membership_view(
            document,
            halves,
            source_matrix,
            mirror_matrix,
            &projected_matrix,
            projection.mapping,
            explicit.then_some(projection),
        )?)
    } else {
        None
    };

    let mut staged = document.clone();
    let mut omitted_parts = BTreeSet::new();
    if let Some(view) = view {
        // Keep retained parts out of the incoming pass and the previous ownership
        // view, so the destructive setter cannot remove their parts or references.
        let staged_old_matrix = staged
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == mirror_matrix.id)
            .ok_or_else(|| "The mirrored matrix disappeared from the staged document".to_owned())?;
        let retained_ids: std::collections::BTreeSet<_> = view
            .retained_cells
            .iter()
            .flat_map(|retained| retained.member_ids.iter().cloned())
            .collect();
        staged_old_matrix
            .part_ids
            .retain(|id| !retained_ids.contains(id));
        if view.normalized_source_matrix.cells.len() != source_matrix.cells.len() {
            crate::matrix::set_matrix(&mut staged, &view.normalized_source_matrix)?;
        }
        crate::matrix::set_matrix(&mut staged, &view.incoming_matrix)?;
        for retained in &view.retained_companion_parts {
            let target = staged
                .parts
                .iter_mut()
                .find(|part| part.id == retained.id)
                .ok_or_else(|| {
                    format!(
                        "The unassociated matrix companion {:?} disappeared",
                        retained.id
                    )
                })?;
            *target = retained.clone();
        }

        {
            let normalized_mirror_matrix = staged
                .matrices
                .iter_mut()
                .find(|matrix| matrix.id == mirror_matrix.id)
                .ok_or_else(|| "The normalized mirrored matrix is unavailable".to_owned())?;
            for retained in &view.retained_cells {
                let normalized_cell = normalized_mirror_matrix
                    .cells
                    .iter_mut()
                    .find(|cell| {
                        cell.row == retained.cell.row && cell.column == retained.cell.column
                    })
                    .ok_or_else(|| {
                        "The normalized retained matrix cell is unavailable".to_owned()
                    })?;
                *normalized_cell = retained.cell.clone();
            }
            let mut ordered_cells: Vec<_> = normalized_mirror_matrix.cells.iter().collect();
            ordered_cells.sort_by_key(|cell| (cell.row, cell.column));
            normalized_mirror_matrix.part_ids =
                ordered_cells
                    .into_iter()
                    .filter(|cell| cell.enabled && !cell.deleted)
                    .flat_map(|cell| {
                        let key_id =
                            format!("matrix/{}/r{}c{}", mirror_matrix.id, cell.row, cell.column);
                        std::iter::once(key_id.clone()).chain(cell.assemblies.iter().map(
                            move |assembly| crate::matrix::companion_id(&key_id, &assembly.id),
                        ))
                    })
                    .collect();
        }
        remove_omitted_matrix_parts(
            &mut staged,
            &mirror_matrix.id,
            &view.omitted_matrix_companions,
        );
        omitted_parts = view.omitted_parts;
    }

    let (source_matrix_members, mirror_matrix_members) = matrix_members_for_pair(document, halves);
    Ok(MatrixStage {
        draft: staged,
        source_members: source_matrix_members,
        mirror_members: mirror_matrix_members,
        omitted_parts,
        allow_absent: exact_matrix_exception
            .is_some_and(|e| e.state == crate::model::MirroredExceptionState::Omitted),
    })
}

/// Project one source matrix into its already assigned mirror identity.
#[doc(hidden)]
pub fn project_matrix(
    source: &Matrix,
    mirror: &Matrix,
    correspondence: &MirroredCorrespondence,
    mapping: Pose2,
) -> Result<Matrix, String> {
    if correspondence.kind != crate::model::MirroredItemKind::Matrix
        || correspondence.source_id != source.id
        || correspondence.mirror_id != mirror.id
    {
        return Err("The matrix correspondence does not match its source and mirror".into());
    }

    crate::matrix::layout::reflected(source, mirror, mapping, false, false)
}

/// Project one source part through its established correspondence, retaining target identifiers.
#[doc(hidden)]
pub fn project_part(
    source: &Part,
    mirror: &Part,
    correspondence: &MirroredCorrespondence,
    mapping: Pose2,
    is_controller: bool,
) -> Result<Part, String> {
    if correspondence.kind != crate::model::MirroredItemKind::Part
        || correspondence.source_id != source.id
        || correspondence.mirror_id != mirror.id
    {
        return Err("The part correspondence does not match its source and mirror".into());
    }

    Ok(super::super::derive::part(
        source,
        &mirror.id,
        &mirror.definition_id,
        &mirror.reference,
        mapping,
        is_controller,
    ))
}
