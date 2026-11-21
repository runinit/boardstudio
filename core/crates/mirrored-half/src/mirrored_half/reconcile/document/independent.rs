//! Independent Part addition staging for forward reconciliation.

use super::*;

/// The read-only reconciliation context independent Part staging derives from.
pub(super) struct IndependentStage<'a> {
    pub(super) document: &'a ProjectDoc,
    pub(super) halves: &'a MirroredHalves,
    pub(super) projection: &'a Projection,
    pub(super) mapping: Pose2,
    pub(super) source_matrix_members: &'a BTreeSet<String>,
    pub(super) mirror_matrix_members: &'a BTreeSet<String>,
}

pub(super) fn stage_independent_additions(
    stage: IndependentStage<'_>,
    staged: &mut ProjectDoc,
    independent_part_targets: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let IndependentStage {
        document,
        halves,
        projection,
        mapping,
        source_matrix_members,
        mirror_matrix_members,
    } = stage;
    let mut sources = BTreeSet::new();
    let mut targets = BTreeSet::new();
    let mut references = BTreeSet::new();
    for PartAddition {
        source_id,
        mirror_id,
        mirror_definition_id,
        mirror_reference,
        ..
    } in &projection.independent_additions
    {
        if !sources.insert(source_id)
            || !targets.insert(mirror_id)
            || !references.insert(mirror_reference)
        {
            return Err(format!(
                "Independent Part addition {source_id:?} repeats a source, mirror identity or reference"
            ));
        }
        if source_matrix_members.contains(source_id) || mirror_matrix_members.contains(mirror_id) {
            return Err("An independent Part addition aliases matrix membership".into());
        }
        if halves.correspondences.iter().any(|correspondence| {
            correspondence.kind == MirroredItemKind::Part
                && (correspondence.source_id == *source_id
                    || correspondence.mirror_id == *mirror_id)
        }) {
            return Err(format!(
                "The independent Part addition {source_id:?} has conflicting pair provenance"
            ));
        }
        if unique_board_owner(document, source_id, "mirrored")? != halves.source_board_id {
            return Err(format!(
                "The independent Part addition {source_id:?} has stale source ownership"
            ));
        }
        let mut sources = document.parts.iter().filter(|part| part.id == *source_id);
        let source = sources
            .next()
            .ok_or_else(|| format!("The independent source Part {source_id:?} is unavailable"))?;
        if sources.next().is_some() {
            return Err(format!(
                "The independent source Part {source_id:?} is ambiguous"
            ));
        }
        let mut definitions = document
            .definitions
            .iter()
            .filter(|definition| definition.id == *mirror_definition_id);
        if definitions.next().is_none() || definitions.next().is_some() {
            return Err(format!(
                "The independent target definition {mirror_definition_id:?} is unavailable or ambiguous"
            ));
        }
        if mirror_id.is_empty() || mirror_reference.is_empty() {
            return Err(format!(
                "The independent target identity {mirror_id:?} and reference must be nonempty"
            ));
        }
        let mut existing_targets = document.parts.iter().filter(|part| part.id == *mirror_id);
        let existing_target = existing_targets.next();
        if existing_targets.next().is_some()
            || document
                .parts
                .iter()
                .any(|part| part.reference == *mirror_reference && part.id != *mirror_id)
        {
            return Err(format!(
                "The independent target identity {mirror_id:?} or reference {mirror_reference:?} is ambiguous or already in use"
            ));
        }
        if existing_target.is_some_and(|part| {
            part.definition_id != *mirror_definition_id || part.reference != *mirror_reference
        }) {
            return Err(format!(
                "The existing independent target {mirror_id:?} does not match its selected definition and reference"
            ));
        }
        if existing_target.is_some() && !projection.additions_applied {
            return Err(format!(
                "The independent target {mirror_id:?} has no prior private Add provenance"
            ));
        }
        if existing_target.is_some_and(|_| {
            unique_board_owner(document, mirror_id, "mirrored").ok()
                != Some(halves.mirror_board_id.as_str())
        }) || (existing_target.is_none()
            && document
                .boards
                .iter()
                .any(|board| board.part_ids.contains(mirror_id)))
        {
            return Err(format!(
                "The independent target {mirror_id:?} has conflicting mirror-board registration"
            ));
        }
        let projected = crate::mirrored_half::derive::part(
            source,
            mirror_id,
            mirror_definition_id,
            mirror_reference,
            mapping,
            crate::mirrored_half::derive::is_controller(
                &document.definitions,
                &source.definition_id,
            ),
        );
        if let Some(staged_target) = staged.parts.iter_mut().find(|part| part.id == *mirror_id) {
            *staged_target = projected;
        } else {
            staged.parts.push(projected);
            let target_board = staged
                .boards
                .iter_mut()
                .find(|board| board.id == halves.mirror_board_id)
                .ok_or_else(|| "The independent target board is unavailable".to_owned())?;
            target_board.part_ids.push(mirror_id.clone());
        }
        independent_part_targets.insert(source_id.clone(), mirror_id.clone());
    }

    Ok(())
}
