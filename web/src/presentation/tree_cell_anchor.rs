use super::*;

#[derive(Clone)]
pub(crate) struct OwnedTreeCellAnchor {
    pub(crate) owner: LayoutOwnerIdentity,
    pub(crate) cell: objects::TreeCellAnchor,
}

pub(crate) fn tree_cell_anchor_for_owner(
    anchor: &Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    owner: &LayoutOwnerIdentity,
) -> Option<objects::TreeCellAnchor> {
    anchor
        .borrow()
        .as_ref()
        .filter(|stored| stored.owner == *owner)
        .map(|stored| stored.cell.clone())
}

pub(crate) fn update_tree_cell_anchor(
    anchor: &Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    owner: &LayoutOwnerIdentity,
    model: &boardstudio_application::ReadModel,
    context: Option<&objects::TreeContext>,
) {
    if owner.workspace != "Layout" && owner.workspace != "PCB" {
        anchor.borrow_mut().take();
        return;
    }
    let (Some(scope), Some(token), Some(revision), Some(context)) =
        (owner.scope.as_ref(), owner.token, owner.revision, context)
    else {
        anchor.borrow_mut().take();
        return;
    };
    if !active_board_scope_matches(model, scope)
        || model.accepted.as_ref().is_none_or(|snapshot| {
            snapshot.token != token || snapshot.document.revision != revision
        })
    {
        anchor.borrow_mut().take();
        return;
    }
    let exact_cell = match context {
        objects::TreeContext::Key {
            matrix_id,
            row,
            column,
        }
        | objects::TreeContext::Component {
            matrix_id: Some(matrix_id),
            row: Some(row),
            column: Some(column),
            ..
        } => Some(objects::TreeCellAnchor {
            matrix_id: matrix_id.clone(),
            row: *row,
            column: *column,
        }),
        _ => None,
    };
    if let Some(cell) = exact_cell {
        if objects::context_for_cell(model, &cell.matrix_id, cell.row, cell.column).is_some() {
            *anchor.borrow_mut() = Some(OwnedTreeCellAnchor {
                owner: owner.clone(),
                cell,
            });
            return;
        }
        anchor.borrow_mut().take();
        return;
    }
    let retained = tree_cell_anchor_for_owner(anchor, owner);
    let compatible = retained.as_ref().is_some_and(|cell| {
        objects::context_for_selection_kind(
            model,
            context,
            objects::LayoutSelectionKind::Key,
            Some(cell),
        )
        .is_some()
    });
    if !compatible {
        anchor.borrow_mut().take();
    }
}
