//! Atomic actions for a primary-key selection spanning several matrices.
use super::{SelectionAdapter, WorkspaceState, objects};
use crate::runtime::Runtime;
use boardstudio_application::{EditResolver, Event, ReadModel, Resolution, Scope, SelectionMode};
use boardstudio_core::model::EditOperation;
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use dioxus::prelude::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

type Cells = BTreeMap<String, Vec<(u32, u32)>>;

#[derive(Clone, PartialEq)]
struct Owner {
    scope: Scope,
    generation: u64,
    context: objects::ScopedTreeContext,
}

#[derive(Clone)]
struct Targets {
    cells: Cells,
    shapes: BTreeMap<String, (u32, u32)>,
    ids: Vec<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Enable,
    Disable,
    Delete,
}

#[derive(Clone, Copy, PartialEq)]
enum Field {
    Enabled,
    Delete,
}

fn owner(
    runtime: &Runtime,
    adapter: &SelectionAdapter,
    workspace: Signal<&'static str>,
) -> Option<Owner> {
    if workspace() != "Layout" {
        return None;
    }
    let scope = runtime.scope()?;
    let context = (adapter.selected_context)()?;
    if context.scope != scope {
        return None;
    }
    Some(Owner {
        scope,
        generation: (adapter.generation)(),
        context,
    })
}

/// Only actual primary MatrixScene members participate. A companion or independent
/// part in the selection makes the entire bulk action ineligible.
fn targets(model: &ReadModel, ids: &[String]) -> Option<Targets> {
    if ids.is_empty() {
        return None;
    }
    let accepted = model.accepted.as_ref()?;
    let mut result = Targets {
        cells: BTreeMap::new(),
        shapes: BTreeMap::new(),
        ids: ids.to_vec(),
    };
    for id in ids {
        let (scene, cell) = accepted.scene.matrix_scenes.iter().find_map(|scene| {
            scene
                .cells
                .iter()
                .find(|cell| {
                    cell.member_id.as_ref() == Some(id)
                        || (!cell.enabled
                            && *id
                                == format!(
                                    "matrix/{}/r{}c{}",
                                    scene.matrix_id, cell.row, cell.column
                                ))
                })
                .map(|cell| (scene, cell))
        })?;
        let matrix = accepted.document.matrices.iter().find(|matrix| {
            matrix.id == scene.matrix_id
                && matrix.board_id.as_deref() == Some(model.active_board_id.as_str())
        })?;
        if matrix
            .cells
            .iter()
            .any(|saved| saved.row == cell.row && saved.column == cell.column && saved.deleted)
        {
            return None;
        }
        result
            .cells
            .entry(matrix.id.clone())
            .or_default()
            .push((cell.row, cell.column));
        result
            .shapes
            .insert(matrix.id.clone(), (matrix.rows, matrix.columns));
    }
    for cells in result.cells.values_mut() {
        cells.sort_unstable();
        cells.dedup();
    }
    Some(result)
}

fn topology_is_current(model: &ReadModel, captured: &Targets) -> bool {
    let Some(accepted) = model.accepted.as_ref() else {
        return false;
    };
    captured.shapes.iter().all(|(id, shape)| {
        accepted.document.matrices.iter().any(|matrix| {
            matrix.id == *id
                && (matrix.rows, matrix.columns) == *shape
                && matrix.board_id.as_deref() == Some(model.active_board_id.as_str())
        })
    }) && targets(model, &captured.ids).is_some_and(|current| current.cells == captured.cells)
}

fn retained_is_current(model: &ReadModel, captured: &Targets) -> bool {
    topology_is_current(model, captured)
        && model
            .selected_part_ids
            .iter()
            .all(|id| captured.ids.contains(id))
        && captured
            .ids
            .iter()
            .filter(|id| !model.selected_part_ids.contains(id))
            .all(|id| {
                model.accepted.as_ref().is_some_and(|accepted| {
                    accepted.scene.matrix_scenes.iter().any(|scene| {
                        scene.cells.iter().any(|cell| {
                            !cell.enabled
                                && *id
                                    == format!(
                                        "matrix/{}/r{}c{}",
                                        scene.matrix_id, cell.row, cell.column
                                    )
                        })
                    })
                })
            })
}

#[component]
pub(super) fn SelectedKeysActions() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let adapter = use_context::<SelectionAdapter>();
    let workspace = use_context::<WorkspaceState>().0;
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let retained = use_hook(|| Rc::new(RefCell::new(None::<(Owner, Targets)>)));
    let selection_lifetime = use_signal(|| 0u64);
    let pending_owner = use_hook(|| Rc::new(RefCell::new(None::<(Owner, u64, Targets)>)));
    let edits = use_hook(PendingEditSignals::<Field>::new);
    let delete_pending = use_signal(|| false);
    let mut enabled_draft = use_signal(String::new);
    let mut enabled_failure = use_signal(|| None::<String>);
    let mut failure = use_signal(|| None::<String>);
    edits.bind_field(Field::Enabled, enabled_draft, enabled_failure);
    edits.bind_one_shot(Field::Delete, delete_pending);
    use_drop({
        let edits = edits.clone();
        move || {
            edits.unbind_field(&Field::Enabled);
            edits.unbind_one_shot(&Field::Delete);
        }
    });
    use_effect({
        let retained = retained.clone();
        let mut selection_lifetime = selection_lifetime;
        let selected_context = adapter.selected_context;
        move || {
            let _selection = selected_context.read();
            retained.borrow_mut().take();
            failure.set(None);
            enabled_failure.set(None);
            let next = (*selection_lifetime.peek()).saturating_add(1);
            selection_lifetime.set(next);
        }
    });
    let observed_selection_lifetime = selection_lifetime();
    let current_owner = owner(&runtime, &adapter, workspace);
    let observation_live =
        pending_owner
            .borrow()
            .as_ref()
            .is_some_and(|(captured, lifetime, _)| {
                current_owner.as_ref() == Some(captured) && observed_selection_lifetime == *lifetime
            });
    for result in edits.settle(observation_live, |_| String::new()) {
        match result {
            PendingEditResult::Failed {
                key: Field::Delete,
                message,
            } => failure.set(Some(message)),
            PendingEditResult::Failed {
                key: Field::Enabled,
                ..
            } => {}
            PendingEditResult::Landed {
                key: Field::Delete, ..
            } => {
                if let Some((_, _, captured)) = pending_owner.borrow().as_ref()
                    && runtime
                        .model()
                        .selected_part_ids
                        .iter()
                        .all(|id| captured.ids.contains(id))
                {
                    let mut selected_context = adapter.selected_context;
                    selected_context.set(None);
                    runtime.submit(Event::SelectParts {
                        operation_id: runtime.operation(),
                        part_ids: vec![],
                        range_part_ids: vec![],
                        mode: SelectionMode::Replace,
                    });
                }
                retained.borrow_mut().take();
                failure.set(None);
            }
            PendingEditResult::Landed {
                key: Field::Enabled,
                ..
            } => {
                if let Some((_, _, captured)) = pending_owner.borrow().as_ref() {
                    let model = runtime.model();
                    let live = captured
                        .ids
                        .iter()
                        .filter(|id| {
                            model.accepted.as_ref().is_some_and(|accepted| {
                                accepted.scene.matrix_scenes.iter().any(|scene| {
                                    scene.cells.iter().any(|cell| {
                                        cell.enabled && cell.member_id.as_ref() == Some(*id)
                                    })
                                })
                            })
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    if !live.is_empty() && model.selected_part_ids != live {
                        runtime.submit(Event::SelectParts {
                            operation_id: runtime.operation(),
                            part_ids: live,
                            range_part_ids: vec![],
                            mode: SelectionMode::Replace,
                        });
                    }
                }
            }
            PendingEditResult::Retired { key: Field::Delete } => failure.set(None),
            PendingEditResult::Retired {
                key: Field::Enabled,
            } => {}
        }
    }
    let model = runtime.model();
    if retained
        .borrow()
        .as_ref()
        .is_some_and(|(held_owner, captured)| {
            current_owner.as_ref() != Some(held_owner) || !retained_is_current(&model, captured)
        })
    {
        retained.borrow_mut().take();
    }
    let projected = current_owner
        .as_ref()
        .and_then(|current| {
            let held = retained.borrow();
            if let Some((held_owner, held_targets)) = held.as_ref()
                && held_owner == current
                && retained_is_current(&model, held_targets)
            {
                return Some(held_targets.clone());
            }
            targets(&model, &model.selected_part_ids)
        })
        .filter(|selection| selection.cells.len() > 1);
    let (Some(current_owner), Some(projected)) = (current_owner, projected) else {
        return rsx! {};
    };
    let count = projected.cells.values().map(Vec::len).sum::<usize>();
    let matrix_count = projected.cells.len();
    let on_action =
        EventHandler::new(move |action: Action| {
            if (action == Action::Delete && delete_pending())
                || owner(&runtime, &adapter, workspace).as_ref() != Some(&current_owner)
            {
                return;
            }
            let lifetime = *selection_lifetime.peek();
            let captured = projected.clone();
            let model = runtime.model();
            let current_targets = retained
                .borrow()
                .as_ref()
                .filter(|(held_owner, held)| {
                    held_owner == &current_owner && retained_is_current(&model, held)
                })
                .map(|(_, held)| held.clone())
                .or_else(|| targets(&model, &model.selected_part_ids));
            if !topology_is_current(&model, &captured)
                || current_targets.is_none_or(|current| current.cells != captured.cells)
            {
                return;
            }
            if pending_owner.borrow().as_ref().is_some_and(
                |(previous_owner, previous_lifetime, _)| {
                    previous_owner != &current_owner || *previous_lifetime != lifetime
                },
            ) {
                // Changing the observation owner never cancels Session's committed intent.
                // Retire its UI observation before the new owner starts another edit.
                edits.settle(false, |_| String::new());
            }
            *retained.borrow_mut() = Some((current_owner.clone(), captured.clone()));
            *pending_owner.borrow_mut() = Some((current_owner.clone(), lifetime, captured.clone()));
            match action {
                Action::Delete => failure.set(None),
                Action::Enable | Action::Disable => enabled_failure.set(None),
            }
            let board_id = current_owner.scope.board_id.clone();
            let resolver = EditResolver::new("selected-keys-action", move |accepted| {
                let accepted_model = ReadModel {
                    accepted: Some(accepted.clone()),
                    active_board_id: board_id.clone(),
                    ..ReadModel::default()
                };
                if !topology_is_current(&accepted_model, &captured) {
                    return Resolution::Retire(
                        "The selected matrix structure changed before this edit ran.".into(),
                    );
                }
                let operation = match action {
                    Action::Enable | Action::Disable => EditOperation::SetMatrixCellsEnabled {
                        cells: captured.cells.clone(),
                        enabled: action == Action::Enable,
                    },
                    Action::Delete => EditOperation::RemoveSelectedMatrixCells {
                        cells: captured.cells.clone(),
                    },
                };
                Resolution::submit(captured.cells.keys().cloned().collect(), operation)
            });
            match action {
                Action::Enable | Action::Disable => {
                    let draft = (action == Action::Enable).to_string();
                    enabled_draft.set(draft.clone());
                    edits.begin_field(
                        &runtime,
                        Field::Enabled,
                        "selected-keys-enabled",
                        Some("layout".into()),
                        resolver,
                        &draft,
                    );
                }
                Action::Delete => edits.begin_one_shot(
                    &runtime,
                    Field::Delete,
                    "selected-keys-delete",
                    Some("layout".into()),
                    resolver,
                ),
            }
        });
    rsx! {
        section { class: "m1-layout-component-inspector", aria_label: "Selected keys across matrices",
            h3 { "{count} keys across {matrix_count} matrices" }
            div { class: "m1-layout-transform-actions",
                button { r#type: "button", aria_label: "Enable selected keys", onclick: move |_| on_action.call(Action::Enable), "Enable" }
                button { r#type: "button", aria_label: "Disable selected keys", onclick: move |_| on_action.call(Action::Disable), "Disable" }
                button { r#type: "button", aria_label: "Delete selected keys", disabled: delete_pending(), onclick: move |_| on_action.call(Action::Delete), "Delete keys" }
            }
            if let Some(message) = enabled_failure().or_else(|| failure()) { p { role: "alert", "{message}" } }
        }
    }
}

#[cfg(test)]
mod tests;
