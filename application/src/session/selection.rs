use super::*;
use std::collections::HashSet;

pub(super) struct MatrixCellSelection {
    pub(super) scope: Scope,
    pub(super) matrix_id: String,
    pub(super) target_part_id: String,
    pub(super) part_ids: Vec<String>,
    pub(super) mode: SelectionMode,
}

impl Session {
    pub(super) fn select_parts(
        &mut self,
        operation_id: OperationId,
        part_ids: Vec<String>,
        range_part_ids: Vec<String>,
        mode: SelectionMode,
        effects: &mut Vec<Effect>,
    ) {
        if let Some(snapshot) = &self.model.accepted {
            let (incoming, valid_order) = {
                let document = crate::resolved_document(&snapshot.document);
                let valid = document
                    .parts
                    .iter()
                    .map(|part| part.id.as_str())
                    .collect::<HashSet<_>>();
                let incoming = part_ids
                    .into_iter()
                    .filter(|id| valid.contains(id.as_str()))
                    .collect::<Vec<_>>();
                let valid_order = if mode == SelectionMode::Range {
                    range_part_ids
                        .iter()
                        .filter(|id| valid.contains(id.as_str()))
                        .cloned()
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                (incoming, valid_order)
            };
            match mode {
                SelectionMode::Range => {
                    let anchor = self.model.selection_anchor_id.as_ref().or(incoming.first());
                    if let (Some(anchor), Some(last)) = (anchor, incoming.last())
                        && let (Some(a), Some(b)) = (
                            valid_order.iter().position(|id| id == anchor),
                            valid_order.iter().position(|id| id == last),
                        )
                    {
                        self.model.selected_part_ids = valid_order[a.min(b)..=a.max(b)].to_vec();
                    }
                }
                _ => update_selection(&mut self.model.selected_part_ids, incoming.clone(), mode),
            }
            self.model.selection_mode = mode;
            if let Some(anchor) = incoming.first() {
                self.model.selection_anchor_id = Some(anchor.clone());
            }
            self.settle(operation_id, TerminalOutcome::Completed, effects);
        } else {
            self.reject(operation_id, "no accepted document is open", effects);
        }
    }

    pub(super) fn select_matrix_cell(
        &mut self,
        operation_id: OperationId,
        selection: MatrixCellSelection,
        effects: &mut Vec<Effect>,
    ) {
        let MatrixCellSelection {
            scope,
            matrix_id,
            target_part_id,
            part_ids,
            mode,
        } = selection;
        let Some(snapshot) = self.model.accepted.as_ref() else {
            self.reject(operation_id, "no accepted document is open", effects);
            return;
        };
        let Some((mut primary_cells, incoming)) = ({
            let document = crate::resolved_document(&snapshot.document);
            let board = document
                .boards
                .iter()
                .find(|board| board.id == scope.board_id);
            let matrix = document.matrices.iter().find(|matrix| {
                matrix.id == matrix_id
                    && matrix
                        .board_id
                        .as_ref()
                        .is_none_or(|id| id == &scope.board_id)
            });
            let scene = snapshot
                .scene
                .matrix_scenes
                .iter()
                .find(|scene| scene.matrix_id == matrix_id);
            board
                .zip(matrix)
                .zip(scene)
                .filter(|_| self.scope_is_current(&scope))
                .map(|((board, matrix), scene)| {
                    let board_parts: HashSet<_> =
                        board.part_ids.iter().map(String::as_str).collect();
                    let document_parts: HashSet<_> =
                        document.parts.iter().map(|part| part.id.as_str()).collect();
                    let owns_part =
                        |id: &str| board_parts.contains(id) && document_parts.contains(id);
                    let primary_cells = scene
                        .cells
                        .iter()
                        .filter_map(|cell| {
                            let id = cell.member_id.as_deref()?;
                            (cell.enabled
                                && matrix.part_ids.iter().any(|member| member == id)
                                && owns_part(id))
                            .then(|| (cell.row, cell.column, id.to_owned()))
                        })
                        .collect::<Vec<_>>();
                    let incoming = part_ids
                        .into_iter()
                        .filter(|id| owns_part(id))
                        .collect::<Vec<_>>();
                    (primary_cells, incoming)
                })
        }) else {
            self.settle(operation_id, TerminalOutcome::Superseded, effects);
            return;
        };
        primary_cells.sort_by_key(|(row, column, _)| (*row, *column));
        let target_is_live = primary_cells.iter().any(|(_, _, id)| id == &target_part_id);
        if !target_is_live {
            self.settle(operation_id, TerminalOutcome::Superseded, effects);
            return;
        }
        let mut retained_anchor = self.model.selection_anchor_id.clone();
        let mut effective_mode = mode;
        let selected = if mode == SelectionMode::Range {
            let cell_at = |part: &str| {
                primary_cells
                    .iter()
                    .find(|(_, _, id)| id == part)
                    .map(|(row, column, _)| (*row, *column))
            };
            if let Some(((anchor_row, anchor_column), (target_row, target_column))) =
                retained_anchor
                    .as_deref()
                    .and_then(cell_at)
                    .zip(cell_at(&target_part_id))
            {
                primary_cells
                    .iter()
                    .filter(|(row, column, _)| {
                        *row >= anchor_row.min(target_row)
                            && *row <= anchor_row.max(target_row)
                            && *column >= anchor_column.min(target_column)
                            && *column <= anchor_column.max(target_column)
                    })
                    .map(|(_, _, id)| id.clone())
                    .collect::<Vec<_>>()
            } else {
                effective_mode = SelectionMode::Replace;
                retained_anchor = Some(target_part_id.clone());
                incoming
            }
        } else {
            retained_anchor = Some(target_part_id.clone());
            incoming
        };
        update_selection(&mut self.model.selected_part_ids, selected, effective_mode);
        self.model.selection_mode = effective_mode;
        self.model.selection_anchor_id = retained_anchor;
        self.settle(operation_id, TerminalOutcome::Completed, effects);
    }

    pub(super) fn reconcile_selection_and_board(&mut self) {
        let Some(snapshot) = &self.model.accepted else {
            return;
        };
        let resolved = crate::resolved_document(&snapshot.document);
        let parts = resolved
            .parts
            .iter()
            .map(|part| part.id.as_str())
            .collect::<HashSet<_>>();
        self.model
            .selected_part_ids
            .retain(|id| parts.contains(id.as_str()));
        if !snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == self.model.active_board_id)
        {
            self.model.active_board_id = snapshot
                .document
                .boards
                .first()
                .map_or_else(String::new, |board| board.id.clone());
        }
        if self.model.active_instance_id.as_ref().is_some_and(|id| {
            !snapshot.document.hardware.as_ref().is_some_and(|h| {
                h.instances
                    .iter()
                    .any(|i| &i.id == id && i.board_id == self.model.active_board_id)
            })
        }) {
            self.model.active_instance_id = None;
        }
    }
}

fn update_selection(selection: &mut Vec<String>, incoming: Vec<String>, mode: SelectionMode) {
    if matches!(mode, SelectionMode::Replace | SelectionMode::Range) {
        *selection = incoming;
        return;
    }
    for id in incoming {
        match selection.iter().position(|current| current == &id) {
            Some(index) if mode == SelectionMode::Toggle => {
                selection.remove(index);
            }
            None => selection.push(id),
            _ => {}
        }
    }
}
