//! Operation-directed projection for edits that author only settings.

use super::super::settings::KeycapEdit;
use super::*;
use crate::model::ElectricalBoardConfiguration;

pub(super) fn project(
    accepted: &ProjectDoc,
    draft: &mut ProjectDoc,
    operation: &EditOperation,
) -> Result<Option<Vec<String>>, String> {
    let keycap = KeycapEdit::for_operation(operation);
    if keycap.is_none()
        && !matches!(
            operation,
            EditOperation::SetWiringMode { .. }
                | EditOperation::SetKeyBinding { .. }
                | EditOperation::EditKeymap { .. }
        )
    {
        return Ok(None);
    }

    // Dispatch for these operations writes only keycaps, keymap, or electrical board
    // settings. The layout and constraint passes can still refresh geometry, membership,
    // registrations and references. Compare their write set, including poses rewritten
    // by layout::resolve without a change report.
    if draft.matrices != accepted.matrices
        || draft.layouts != accepted.layouts
        || draft.parts != accepted.parts
        || draft.boards != accepted.boards
        || draft.nets != accepted.nets
        || draft.constraints != accepted.constraints
        || draft.outline != accepted.outline
        || !draft
            .hardware
            .iter()
            .flat_map(|hardware| &hardware.pairs)
            .eq(accepted
                .hardware
                .iter()
                .flat_map(|hardware| &hardware.pairs))
        || crate::mechanical::features::validate(accepted).is_err()
    {
        return Ok(None);
    }
    // The lifecycle kernel checks for native/legacy matrix overlap, even on a
    // settings edit. Keep that check for documents with legacy layout links.
    if keycap.is_none()
        && accepted
            .layouts
            .iter()
            .any(|layout| layout.mirror_link.is_some())
    {
        return Ok(None);
    }
    let mut halves = accepted
        .hardware
        .iter()
        .flat_map(|hardware| &hardware.pairs)
        .filter_map(|pair| pair.halves.as_ref());
    for pair in halves.clone() {
        if pair.source_board_id == pair.mirror_board_id {
            return Ok(None);
        }
        // ReplaceDocument skips pair reconciliation for matrixless halves. Their next
        // ordinary edit still runs lifecycle projection to refresh the accepted baseline.
        if keycap.is_none()
            && !pair
                .correspondences
                .iter()
                .any(|link| link.kind == MirroredItemKind::Matrix)
        {
            return Ok(None);
        }
        super::super::validate_local_circuit_targets(accepted, pair)?;
    }

    if let Some(edit) = keycap {
        let Some(settings) = &mut draft.keycaps else {
            return Ok(None);
        };
        edit.project(accepted, settings);
        return Ok(Some(Vec::new()));
    }

    let board_id = match operation {
        EditOperation::SetWiringMode { board_id, .. } => {
            let target = halves.find(|pair| {
                pair.source_board_id == *board_id
                    && pair.correspondences.iter().any(|link| {
                        link.kind == MirroredItemKind::ElectricalBoard
                            && link.source_id == *board_id
                            && link.mirror_id == pair.mirror_board_id
                            && pair.overlay_of(link).is_none()
                    })
            });
            if let Some(pair) = target
                && let Some(mode) = electrical_board(draft, board_id).map(|board| board.mode)
                && let Some(mirror) = draft
                    .hardware
                    .iter_mut()
                    .flat_map(|hardware| &mut hardware.boards)
                    .find(|board| board.board_id == pair.mirror_board_id)
            {
                mirror.mode = mode;
            }
            board_id
        }
        // Binding edits stay on the named half. Legacy bindings write an electrical row
        // as well as the document-level keymap.
        EditOperation::SetKeyBinding { board_id, .. } => board_id,
        EditOperation::EditKeymap { .. } => return Ok(Some(Vec::new())),
        _ => unreachable!(),
    };
    // The full change report includes the requested electrical board if its row changed.
    let changed = (electrical_board(accepted, board_id) != electrical_board(draft, board_id))
        .then(|| board_id.clone())
        .into_iter()
        .collect();
    Ok(Some(changed))
}

fn electrical_board<'a>(
    document: &'a ProjectDoc,
    board_id: &str,
) -> Option<&'a ElectricalBoardConfiguration> {
    document
        .hardware
        .as_ref()?
        .boards
        .iter()
        .find(|board| board.board_id == board_id)
}
